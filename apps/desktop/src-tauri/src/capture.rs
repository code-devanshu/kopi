//! Runs the clipboard watcher, stores what it captures and tells the UI.

use std::sync::Arc;

use kopi_core::privacy::{PrivacyFilter, PrivacySettings};
use kopi_core::storage::{Platform, StorageResult, Store};
use tauri::{AppHandle, Manager};

/// The platform whose excluded apps apply.
#[cfg(not(target_os = "windows"))]
pub const PLATFORM: Platform = Platform::MacOs;
#[cfg(target_os = "windows")]
pub const PLATFORM: Platform = Platform::Windows;

/// Sets up the privacy filter from the settings, which Settings then changes through the
/// app's [`PrivacyFilter`], and starts watching the clipboard. Only copies made from now on
/// are captured.
pub fn start(app: &AppHandle, store: Arc<Store>) -> Result<(), Box<dyn std::error::Error>> {
    let filter = PrivacyFilter::new(PrivacySettings::default());
    reload(&filter, &store)?;
    app.manage(filter.clone());
    watch(app, store, filter)
}

/// Reads the kinds to capture and the excluded apps into `filter`.
pub fn reload(filter: &PrivacyFilter, store: &Store) -> StorageResult<()> {
    let kinds = store.capture_kinds()?;
    let excluded = store.excluded_apps(PLATFORM)?;
    filter.update(|settings| {
        settings.kinds = kinds;
        settings.set_excluded_apps(excluded.iter().map(|app| &app.app_id));
    });
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn watch(
    app: &AppHandle,
    store: Arc<Store>,
    filter: PrivacyFilter,
) -> Result<(), Box<dyn std::error::Error>> {
    use kopi_core::classifier::{self, Saved};
    use kopi_core::retention::Retention;
    use kopi_core::watcher::{WatchEvent, Watcher, WatcherConfig};
    use tauri_specta::Event;

    use crate::events::{ClipAdded, ClipUpdated};

    // The listener runs the filter's second stage. Clones share the settings.
    let saver = filter.clone();
    let handle = app.clone();

    let listener = move |event: WatchEvent| {
        // Kopi's own writes are pastes, which already moved their clip to the top. Settings
        // asks for the access state itself. Skipped and missed copies were never read.
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
        deps(),
        WatcherConfig::new(app.config().identifier.clone()),
        Box::new(filter),
        Box::new(listener),
    )?;
    app.manage(Capture { _watcher: watcher });
    Ok(())
}

#[cfg(target_os = "macos")]
fn deps() -> kopi_core::watcher::WatcherDeps {
    use kopi_platform::macos::{MacForegroundApp, MacPasteboard, MacUserIdle};

    kopi_core::watcher::WatcherDeps {
        pasteboard: Box::new(MacPasteboard::general()),
        apps: Box::new(MacForegroundApp),
        idle: Box::new(MacUserIdle),
    }
}

#[cfg(target_os = "windows")]
fn deps() -> kopi_core::watcher::WatcherDeps {
    use kopi_platform::windows::{WinForegroundApp, WinPasteboard, WinUserIdle};

    kopi_core::watcher::WatcherDeps {
        pasteboard: Box::new(WinPasteboard::general()),
        apps: Box::new(WinForegroundApp::default()),
        idle: Box::new(WinUserIdle),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn watch(
    _app: &AppHandle,
    _store: Arc<Store>,
    _filter: PrivacyFilter,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

/// Keeps the watcher running for as long as the app does.
#[cfg(any(target_os = "macos", target_os = "windows"))]
struct Capture {
    _watcher: kopi_core::watcher::Watcher,
}
