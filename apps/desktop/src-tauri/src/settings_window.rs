//! The Settings window. Unlike the panel it's an ordinary window: built when it's opened,
//! destroyed when it's closed.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// The Settings window's label.
pub const SETTINGS: &str = "settings";

const WIDTH: f64 = 640.0;
const HEIGHT: f64 = 580.0;

/// Opens Settings, or brings it to the front, and hides the panel.
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    crate::panel::hide(app);
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.unminimize();
        window.show()?;
        return window.set_focus();
    }
    let window = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title("Kopi Settings")
        .inner_size(WIDTH, HEIGHT)
        .min_inner_size(WIDTH, 420.0)
        .maximizable(false)
        .center()
        .build()?;
    // Kopi has no Dock icon, so the window has to bring the app forward itself.
    window.set_focus()
}

/// Whether Settings has keyboard focus.
pub fn is_focused(app: &AppHandle) -> bool {
    app.get_webview_window(SETTINGS)
        .is_some_and(|window| window.is_focused().unwrap_or(false))
}
