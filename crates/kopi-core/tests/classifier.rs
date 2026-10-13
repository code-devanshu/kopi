//! A capture through `classifier::save`: classify, the privacy filter's second stage, and the
//! store's upsert by content hash.

use kopi_core::classifier::{Saved, UNRECOGNISED, save};
use kopi_core::privacy::{KindSet, PrivacyFilter, PrivacySettings, reason};
use kopi_core::storage::{ClipKind, MAX_TEXT_INDEX_BYTES, Store};
use kopi_core::watcher::Capture;
use kopi_platform::{AppRef, RawFormat, uti};
use serde_json::json;

/// Formats, then the kind, plain text and metadata they should make.
type Case<'a> = (
    &'a [(&'a str, &'a [u8])],
    ClipKind,
    Option<&'a str>,
    Option<serde_json::Value>,
);

struct Fixture {
    _dir: tempfile::TempDir,
    store: Store,
    filter: PrivacyFilter,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        Self {
            _dir: dir,
            store,
            filter: PrivacyFilter::default(),
        }
    }

    fn copy(&self, app: &str, formats: &[(&str, &[u8])]) -> Saved {
        let capture = Capture {
            change_count: 1,
            source: Some(AppRef {
                bundle_id: Some(format!("com.example.{app}")),
                name: Some(app.to_owned()),
            }),
            formats: formats
                .iter()
                .map(|(format, data)| RawFormat {
                    item_index: 0,
                    format: (*format).to_owned(),
                    data: data.to_vec(),
                })
                .collect(),
        };
        save(&self.store, &self.filter, capture).unwrap()
    }

    /// The clip's id, and whether it was new.
    fn stored(&self, app: &str, formats: &[(&str, &[u8])]) -> (i64, bool) {
        match self.copy(app, formats) {
            Saved::Stored { clip, .. } => (clip.id, clip.inserted),
            Saved::Dropped(reason) => panic!("dropped: {reason}"),
        }
    }
}

#[test]
fn copying_again_moves_the_clip_to_the_top() {
    let f = Fixture::new();
    let (id, inserted) = f.stored("Notes", &[(uti::PLAIN_TEXT, b"Hello")]);
    assert!(inserted);
    let (again, inserted) = f.stored("Mail", &[(uti::PLAIN_TEXT, b"Hello")]);
    assert_eq!((again, inserted), (id, false));

    let clip = f.store.get_clip(id).unwrap().unwrap();
    assert_eq!(clip.use_count, 2);
    assert_eq!(clip.source_app_name.as_deref(), Some("Mail"));
    assert_eq!(clip.source_app_id.as_deref(), Some("com.example.Mail"));
}

#[test]
fn plain_and_rich_copies_of_the_same_words_are_two_clips() {
    let f = Fixture::new();
    let (plain, _) = f.stored("Notes", &[(uti::PLAIN_TEXT, b"Hello")]);
    let (rich, inserted) = f.stored(
        "Safari",
        &[(uti::PLAIN_TEXT, b"Hello"), (uti::HTML, b"<b>Hello</b>")],
    );
    assert!(inserted);
    assert_ne!(plain, rich);
    assert_eq!(
        f.store.get_clip(rich).unwrap().unwrap().kind,
        ClipKind::RichText
    );
}

#[test]
fn rich_text_keeps_the_latest_formatting() {
    let f = Fixture::new();
    let (id, _) = f.stored(
        "Safari",
        &[(uti::PLAIN_TEXT, b"Hello"), (uti::HTML, b"<b>Hello</b>")],
    );
    let (again, inserted) = f.stored(
        "Pages",
        &[
            (uti::PLAIN_TEXT, b"Hello"),
            (uti::RTF, b"{\\rtf1 \\i Hello}"),
        ],
    );
    assert_eq!((again, inserted), (id, false));
    let formats: Vec<String> = f
        .store
        .clip_formats(id)
        .unwrap()
        .into_iter()
        .map(|f| f.format)
        .collect();
    assert_eq!(formats, [uti::RTF]);
}

#[test]
fn kinds_and_metadata_are_stored() {
    let f = Fixture::new();
    let cases: &[Case] = &[
        (
            &[(uti::PLAIN_TEXT, b"https://kopi.devanshuverma.in/")],
            ClipKind::Link,
            Some("https://kopi.devanshuverma.in/"),
            Some(json!({ "host": "kopi.devanshuverma.in" })),
        ),
        (
            &[(uti::PLAIN_TEXT, b"#0f0")],
            ClipKind::Color,
            Some("#00FF00FF"),
            Some(json!({ "rgba": [0, 255, 0, 255] })),
        ),
        (
            &[(uti::PLAIN_TEXT, b"def f(x):\n    return x * 2\n")],
            ClipKind::Code,
            Some("def f(x):\n    return x * 2\n"),
            Some(json!({ "lang": "python" })),
        ),
        (
            &[
                (uti::FILE_URL, b"file:///Users/me/Report%202026.pdf"),
                (uti::PLAIN_TEXT, b"Report 2026.pdf"),
            ],
            ClipKind::File,
            Some("Report 2026.pdf"),
            Some(json!({ "count": 1, "names": ["Report 2026.pdf"] })),
        ),
    ];
    for (formats, kind, text, metadata) in cases {
        let (id, _) = f.stored("App", formats);
        let clip = f.store.get_clip(id).unwrap().unwrap();
        assert_eq!(clip.kind, *kind);
        assert_eq!(clip.plain_text.as_deref(), *text, "{kind}");
        assert_eq!(clip.metadata, *metadata, "{kind}");
    }
}

#[test]
fn long_text_is_cut_for_search_and_kept_whole_for_paste() {
    let f = Fixture::new();
    let text = "word ".repeat(MAX_TEXT_INDEX_BYTES / 4);
    let (id, _) = f.stored("Notes", &[(uti::PLAIN_TEXT, text.as_bytes())]);
    let clip = f.store.get_clip(id).unwrap().unwrap();
    assert_eq!(clip.plain_text.unwrap().len(), MAX_TEXT_INDEX_BYTES);
    let formats = f.store.clip_formats(id).unwrap();
    assert_eq!(formats[0].data, text.as_bytes());
}

#[test]
fn the_privacy_filter_has_the_last_word() {
    let f = Fixture::new();
    f.filter.update(|s: &mut PrivacySettings| {
        s.kinds = KindSet::ALL.without(ClipKind::Link);
        s.max_clip_bytes = 64;
    });
    // Only the content shows it's a link.
    assert_eq!(
        f.copy("Notes", &[(uti::PLAIN_TEXT, b"https://example.com")]),
        Saved::Dropped(reason::KIND_OFF)
    );
    assert_eq!(
        f.copy("Notes", &[(uti::PLAIN_TEXT, &[b'a'; 65])]),
        Saved::Dropped(reason::TOO_LARGE)
    );
    assert_eq!(
        f.copy("Notes", &[("com.example.private", b"x")]),
        Saved::Dropped(UNRECOGNISED)
    );
    assert!(matches!(
        f.copy("Notes", &[(uti::PLAIN_TEXT, b"fine")]),
        Saved::Stored {
            kind: ClipKind::Text,
            ..
        }
    ));
}
