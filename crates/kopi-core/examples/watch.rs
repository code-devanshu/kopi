//! Watches the real clipboard, and classifies and stores each copy in a temporary store. Prints
//! the format names, sizes, source app, and the kind the classifier picked, or that the copy was
//! a duplicate and moved to the top. Clipboard content is never printed.
//!
//! ```sh
//! cargo run -p kopi-core --example watch
//! ```
//!
//! Copy something in any app and a line appears. Ctrl+C stops it. The watcher runs on its own
//! thread and the main thread runs the AppKit loop, as in the app.
//!
//! The privacy filter runs with its defaults: the concealed, transient and auto-generated
//! markers, and the default excluded apps, taken from a fresh store in a temporary folder.
//! Copies it turns down are reported as skipped and never read. Copy the same thing twice to see
//! the second one land on the first clip.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This example is macOS only.");
}

#[cfg(target_os = "macos")]
fn main() {
    use kopi_core::classifier::{self, Saved};
    use kopi_core::privacy::{PrivacyFilter, PrivacySettings};
    use kopi_core::storage::{Platform, Store};
    use kopi_core::watcher::{WatchEvent, Watcher, WatcherConfig, WatcherDeps};
    use kopi_platform::macos::{MacForegroundApp, MacPasteboard, MacUserIdle};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let dir = tempfile::tempdir().expect("failed to make a temporary folder");
    let store = Store::open(dir.path()).expect("failed to open the store");
    let excluded = store
        .excluded_apps(Platform::MacOs)
        .expect("failed to read the excluded apps");
    let mut settings = PrivacySettings::default();
    settings.set_excluded_apps(excluded.iter().map(|app| &app.app_id));
    let filter = PrivacyFilter::new(settings);
    // The listener runs the second stage. Clones share the settings.
    let saver = filter.clone();
    println!("Excluded apps:");
    for app in &excluded {
        println!("    {} ({})", app.app_name, app.app_id);
    }

    let mut copies = 0;
    let listener = move |event: WatchEvent| match event {
        WatchEvent::Copied(capture) => {
            copies += 1;
            let source = capture
                .source
                .as_ref()
                .map_or("unknown app".to_owned(), |app| {
                    match (&app.name, &app.bundle_id) {
                        (Some(name), Some(id)) => format!("{name} ({id})"),
                        (name, id) => name.as_ref().or(id.as_ref()).cloned().unwrap_or_default(),
                    }
                });
            println!(
                "copy {copies} from {source}, change {}:",
                capture.change_count
            );
            for format in &capture.formats {
                println!(
                    "    item {}  {}  {} bytes",
                    format.item_index,
                    format.format,
                    format.data.len()
                );
            }
            match classifier::save(&store, &saver, capture) {
                Ok(Saved::Stored { kind, clip }) if clip.inserted => {
                    println!("    new {kind} clip {}", clip.id);
                }
                Ok(Saved::Stored { kind, clip }) => {
                    println!("    duplicate of {kind} clip {}, moved to the top", clip.id);
                }
                Ok(Saved::Dropped(reason)) => println!("    not stored ({reason})"),
                Err(error) => println!("    failed to store: {error}"),
            }
        }
        WatchEvent::OwnWrite { change_count } => {
            println!("Kopi's own write, change {change_count}: ignored");
        }
        WatchEvent::Skipped {
            change_count,
            reason,
        } => println!("skipped change {change_count} ({reason}): nothing read"),
        WatchEvent::Missed {
            change_count,
            error,
        } => println!("missed change {change_count}: {error}"),
        WatchEvent::AccessChanged(access) => println!("clipboard access: {access:?}"),
    };

    let deps = WatcherDeps {
        pasteboard: Box::new(MacPasteboard::general()),
        apps: Box::new(MacForegroundApp),
        idle: Box::new(MacUserIdle),
    };
    let _watcher = Watcher::spawn(
        deps,
        WatcherConfig::new("in.devanshuverma.kopi"),
        Box::new(filter),
        Box::new(listener),
    )
    .expect("failed to start the watcher");
    println!("Watching the clipboard. Copy something, or press Ctrl+C to stop.");

    let mtm = MainThreadMarker::new().expect("main runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    // No Dock icon, like Kopi.
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.run();
}
