//! The onboarding window's commands. It also uses the permission, shortcut and launch at login
//! commands that Settings does.

use std::sync::Arc;

use kopi_core::storage::Store;
use tauri::{AppHandle, Manager, State};

use super::clips::blocking;
use crate::onboarding::ONBOARDING;

/// Where onboarding's "How Kopi handles your clips" link goes.
const PRIVACY_DESIGN: &str =
    "https://github.com/code-devanshu/kopi/blob/main/docs/PRIVACY_DESIGN.md";

/// Saves that onboarding is done and closes it. With `open_panel`, shows the panel after.
#[tauri::command]
#[specta::specta]
pub async fn finish_onboarding(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    open_panel: bool,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    blocking(move || store.set_onboarded(true)).await?;
    if let Some(window) = app.get_webview_window(ONBOARDING) {
        window.destroy().map_err(|e| e.to_string())?;
    }
    if open_panel {
        crate::panel::show(&app);
    }
    Ok(())
}

/// Opens Kopi's privacy design in the browser.
#[tauri::command]
#[specta::specta]
pub async fn open_privacy_design() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(not(target_os = "macos"))]
    let opener = "explorer";
    // Waits for the opener, which hands the link to the browser and exits, so it's reaped.
    // Its exit code isn't checked: explorer returns 1 even when it opened the link.
    tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new(opener)
            .arg(PRIVACY_DESIGN)
            .status()
    })
    .await
    .map_err(|e| e.to_string())?
    .map(drop)
    .map_err(|e| e.to_string())
}
