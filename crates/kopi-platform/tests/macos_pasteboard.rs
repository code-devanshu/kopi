//! `MacPasteboard` against a real, private pasteboard. The user's clipboard is never touched.
//!
//! AppKit wants pasteboard calls on the main thread, and libtest runs tests on worker threads,
//! so this file has its own `main` (`harness = false` in `Cargo.toml`).

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use kopi_platform::macos::MacPasteboard;
    use kopi_platform::{Pasteboard, PasteboardAccess, PasteboardError, RawFormat, uti};
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
    use objc2_foundation::{NSArray, NSData, NSString};

    type Test = (&'static str, fn(&Fixture));

    const TESTS: &[Test] = &[
        ("types_lists_each_item", types_lists_each_item),
        (
            "reads_only_the_formats_asked_for",
            reads_only_the_formats_asked_for,
        ),
        (
            "stale_change_count_reads_nothing",
            stale_change_count_reads_nothing,
        ),
        ("over_the_limit_reads_nothing", over_the_limit_reads_nothing),
        ("change_count_follows_writes", change_count_follows_writes),
    ];

    pub fn run() {
        let fixture = Fixture::new();
        for (name, test) in TESTS {
            test(&fixture);
            println!("test {name} ... ok");
        }
        fixture.release();
        println!("\n{} passed", TESTS.len());
    }

    struct Fixture {
        native: Retained<NSPasteboard>,
        pasteboard: MacPasteboard,
    }

    impl Fixture {
        fn new() -> Self {
            let native = NSPasteboard::pasteboardWithUniqueName();
            let pasteboard = MacPasteboard::named(native.name().to_string());
            Self { native, pasteboard }
        }

        /// Replaces the contents with one pasteboard item per entry.
        fn write(&self, items: &[&[(&str, &[u8])]]) -> u64 {
            let objects: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = items
                .iter()
                .map(|formats| {
                    let item = NSPasteboardItem::new();
                    for (format, data) in *formats {
                        assert!(item.setData_forType(
                            &NSData::with_bytes(data),
                            &NSString::from_str(format)
                        ));
                    }
                    ProtocolObject::from_retained(item)
                })
                .collect();
            self.native.clearContents();
            assert!(
                self.native
                    .writeObjects(&NSArray::from_retained_slice(&objects))
            );
            self.native.changeCount() as u64
        }

        fn release(self) {
            // Not in objc2-app-kit's bindings. Frees the pasteboard in the pasteboard server.
            // SAFETY: `releaseGlobally` takes no arguments and returns nothing.
            let () = unsafe { msg_send![&*self.native, releaseGlobally] };
        }
    }

    fn two_items(f: &Fixture) -> u64 {
        f.write(&[
            &[(uti::PLAIN_TEXT, b"hello"), (uti::HTML, b"<b>hello</b>")],
            &[
                (uti::FILE_URL, b"file:///tmp/a.txt"),
                ("com.example.private", b"x"),
            ],
        ])
    }

    fn types_lists_each_item(f: &Fixture) {
        let count = two_items(f);
        let types = f.pasteboard.types().unwrap();
        assert_eq!(types.change_count, count);
        assert_eq!(types.items.len(), 2);
        assert!(types.items[0].iter().any(|t| t == uti::PLAIN_TEXT));
        assert!(types.items[0].iter().any(|t| t == uti::HTML));
        assert!(types.items[1].iter().any(|t| t == uti::FILE_URL));
        assert!(types.contains("com.example.private"));
        assert!(!types.contains(uti::PNG));
    }

    fn reads_only_the_formats_asked_for(f: &Fixture) {
        let count = two_items(f);
        assert_eq!(
            f.pasteboard.access().unwrap(),
            PasteboardAccess::AlwaysAllow
        );
        let read = f
            .pasteboard
            .read(
                count,
                &[
                    vec![uti::PLAIN_TEXT.into(), uti::PNG.into()],
                    vec![uti::FILE_URL.into()],
                ],
                1024,
            )
            .unwrap();
        assert_eq!(
            read,
            vec![
                RawFormat {
                    item_index: 0,
                    format: uti::PLAIN_TEXT.into(),
                    data: b"hello".to_vec(),
                },
                RawFormat {
                    item_index: 1,
                    format: uti::FILE_URL.into(),
                    data: b"file:///tmp/a.txt".to_vec(),
                },
            ]
        );
    }

    fn stale_change_count_reads_nothing(f: &Fixture) {
        let count = two_items(f);
        let formats = [vec![uti::PLAIN_TEXT.to_string()]];
        assert_eq!(
            f.pasteboard.read(count - 1, &formats, 1024),
            Err(PasteboardError::Changed)
        );
    }

    fn over_the_limit_reads_nothing(f: &Fixture) {
        let count = two_items(f);
        let formats = [vec![uti::PLAIN_TEXT.to_string(), uti::HTML.to_string()]];
        // "hello" fits, "<b>hello</b>" takes it over.
        assert_eq!(
            f.pasteboard.read(count, &formats, 10),
            Err(PasteboardError::TooLarge)
        );
        assert_eq!(f.pasteboard.read(count, &formats, 17).unwrap().len(), 2);
    }

    fn change_count_follows_writes(f: &Fixture) {
        let first = two_items(f);
        assert_eq!(f.pasteboard.change_count().unwrap(), first);
        let second = f.write(&[&[(uti::PLAIN_TEXT, b"again")]]);
        assert!(second > first);
        assert_eq!(f.pasteboard.change_count().unwrap(), second);
    }
}
