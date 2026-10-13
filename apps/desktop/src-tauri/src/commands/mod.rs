//! Tauri commands. They stay thin and call into `kopi-core`.

pub mod clips;
pub mod paste;
pub mod search;

use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub os: Os,
}

#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Macos,
    Windows,
    Other,
}

impl From<kopi_core::Os> for Os {
    fn from(os: kopi_core::Os) -> Self {
        match os {
            kopi_core::Os::MacOs => Os::Macos,
            kopi_core::Os::Windows => Os::Windows,
            kopi_core::Os::Other => Os::Other,
        }
    }
}

/// Kopi's version and the OS it's running on.
#[tauri::command]
#[specta::specta]
pub fn app_info() -> AppInfo {
    AppInfo {
        version: kopi_core::VERSION.to_owned(),
        os: kopi_core::current_os().into(),
    }
}

/// Whether Kopi starts at login.
#[tauri::command]
#[specta::specta]
pub fn launch_at_login(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Turns launch at login on or off.
#[tauri::command]
#[specta::specta]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autostart = app.autolaunch();
    let result = if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    };
    result.map_err(|e| e.to_string())
}

/// Hides the panel: Esc, or after a paste.
#[tauri::command]
#[specta::specta]
pub fn hide_panel(app: AppHandle) {
    crate::panel::hide(&app);
}
