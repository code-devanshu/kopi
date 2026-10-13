//! Runs the clipboard watcher, stores what it captures and tells the UI.
//!
//! macOS only for now. The Windows watcher arrives with the Windows alpha.

use std::sync::Arc;

use kopi_core::storage::Store;
use tauri::AppHandle;

/// Starts watching the clipboard. Only copies made from now on are captured.
#[cfg(target_os = "macos")]
pub fn start(app: &AppHandle, store: Arc<Store>) -> Result<(), Box<dyn std::error::Error>> {
    use kopi_core::classifier::{self, Saved};
    use kopi_core::privacy::{PrivacyFilter, PrivacySettings};
    use kopi_core::retention::Retention;
    use kopi_core::storage::Platform;
    use kopi_core::watcher::{WatchEvent, Watcher, WatcherConfig, WatcherDeps};
    use kopi_platform::macos::{MacForegroundApp, MacPasteboard, MacUserIdle};
    use tauri::Manager;
    use tauri_specta::Event;

    use crate::events::{ClipAdded, ClipUpdated};

    let mut settings = PrivacySettings::default();
    let excluded = store.excluded_apps(Platform::MacOs)?;
    settings.set_excluded_apps(excluded.iter().map(|app| &app.app_id));
    let filter = PrivacyFilter::new(settings);
    // The listener runs the filter's second stage. Clones share the settings.
    let saver = filter.clone();
    let handle = app.clone();

    let listener = move |event: WatchEvent| {
        // Kopi's own writes are pastes, which already moved their clip to the top. The access
        // state is for onboarding (#16). Skipped and missed copies were never read.
        let WatchEvent::Copied(capture) = event else {
            return;
        };
        match classifier::save(&store, &saver, capture) {
            Ok(Saved::Stored { clip, .. }) if clip.inserted => {
                let _ = ClipAdded { id: clip.id }.emit(&handle);
                handle.state::<Retention>().clips_added();
            }
            Ok(Saved::Stored { clip, .. }) => {
                let _ = ClipUpdated { id: clip.id }.emit(&handle);
            }
            Ok(Saved::Dropped(_)) => {}
            // Storage errors name the database or a file, never clip content.
            Err(error) => eprintln!("kopi: couldn't store a copy: {error}"),
        }
    };
    let watcher = Watcher::spawn(
        WatcherDeps {
            pasteboard: Box::new(MacPasteboard::general()),
            apps: Box::new(MacForegroundApp),
            idle: Box::new(MacUserIdle),
        },
        WatcherConfig::new(app.config().identifier.clone()),
        Box::new(filter.clone()),
        Box::new(listener),
    )?;
    app.manage(Capture {
        _watcher: watcher,
        _filter: filter,
    });
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn start(_app: &AppHandle, _store: Arc<Store>) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

/// Keeps the watcher running for as long as the app does. The filter is here for pausing and
/// the excluded apps settings (#16).
#[cfg(target_os = "macos")]
struct Capture {
    _watcher: kopi_core::watcher::Watcher,
    _filter: kopi_core::privacy::PrivacyFilter,
}
