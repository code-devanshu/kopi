//! The first-run window: permissions, the shortcut and launch at login. It opens at launch
//! until the user finishes or closes it, and from the menu bar after that. Like Settings, it's
//! built when it's opened and destroyed when it's closed.

use std::sync::Arc;

use kopi_core::storage::Store;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// The onboarding window's label.
pub const ONBOARDING: &str = "onboarding";

const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 520.0;

/// Opens onboarding, or brings it to the front, and hides the panel.
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    crate::panel::hide(app);
    if let Some(window) = app.get_webview_window(ONBOARDING) {
        let _ = window.unminimize();
        window.show()?;
        return window.set_focus();
    }
    let window =
        WebviewWindowBuilder::new(app, ONBOARDING, WebviewUrl::App("onboarding.html".into()))
            .title("Welcome to Kopi")
            .inner_size(WIDTH, HEIGHT)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .center()
            .build()?;
    // Kopi has no Dock icon, so the window has to bring the app forward itself.
    window.set_focus()
}

/// Opens onboarding if the user hasn't been through it.
pub fn open_on_first_run(app: &AppHandle, store: &Store) {
    match store.onboarded() {
        Ok(true) => {}
        // An unreadable flag reads as done: better to skip onboarding than show it every time.
        Err(error) => eprintln!("kopi: couldn't read the onboarding setting: {error}"),
        Ok(false) => {
            if let Err(error) = open(app) {
                eprintln!("kopi: couldn't open onboarding: {error}");
            }
        }
    }
}

/// Remembers that onboarding is done, off the main thread, so it doesn't open at the next
/// launch.
pub fn mark_done(app: &AppHandle) {
    let store = Arc::clone(&app.state::<Arc<Store>>());
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = store.set_onboarded(true) {
            eprintln!("kopi: couldn't save the onboarding setting: {error}");
        }
    });
}

/// Whether onboarding has keyboard focus.
pub fn is_focused(app: &AppHandle) -> bool {
    app.get_webview_window(ONBOARDING)
        .is_some_and(|window| window.is_focused().unwrap_or(false))
}
