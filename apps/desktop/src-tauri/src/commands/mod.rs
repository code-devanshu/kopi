//! Tauri commands. They stay thin and call into `kopi-core`.

pub mod clips;
pub mod history;
pub mod onboarding;
pub mod paste;
pub mod pinboards;
pub mod search;
pub mod settings;
pub mod sync;

use serde::Serialize;
use specta::Type;
use tauri::AppHandle;

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
    crate::login_item::is_enabled(&app)
}

/// Turns launch at login on or off.
#[tauri::command]
#[specta::specta]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    crate::login_item::set_enabled(&app, enabled)
}

/// Hides the panel: Esc, or after a paste.
#[tauri::command]
#[specta::specta]
pub fn hide_panel(app: AppHandle) {
    crate::panel::hide(&app);
}
