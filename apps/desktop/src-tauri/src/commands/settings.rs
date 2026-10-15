//! The Settings window's commands: the shortcut, what to capture, the theme, updates and
//! permissions.
//! The history limits are in `history`, and launch at login is in `commands`.

use std::sync::Arc;

use kopi_core::privacy::{KindSet, PrivacyFilter};
use kopi_core::settings;
use kopi_core::storage::{self, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::clips::{SourceApp, blocking};
use super::search::ClipKind;
use crate::events::ThemeChanged;
use crate::{capture, hotkeys, settings_window};

/// The settings that aren't in a command of their own.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// The shortcut that opens the panel, as the shortcut plugin writes it: `shift+super+KeyV`.
    pub shortcut: String,
    /// The kinds of clip to capture.
    pub capture_kinds: Vec<ClipKind>,
    pub theme: Theme,
    /// Whether Kopi checks for updates by itself and installs them.
    pub auto_update: bool,
}

/// The app's color scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follows the system's light or dark mode.
    System,
    Light,
    Dark,
}

impl From<settings::Theme> for Theme {
    fn from(theme: settings::Theme) -> Self {
        match theme {
            settings::Theme::System => Self::System,
            settings::Theme::Light => Self::Light,
            settings::Theme::Dark => Self::Dark,
        }
    }
}

impl From<Theme> for settings::Theme {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::System => Self::System,
            Theme::Light => Self::Light,
            Theme::Dark => Self::Dark,
        }
    }
}

impl From<Theme> for Option<tauri::Theme> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::System => None,
            Theme::Light => Some(tauri::Theme::Light),
            Theme::Dark => Some(tauri::Theme::Dark),
        }
    }
}

/// An app Kopi never captures from.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedApp {
    /// A macOS bundle ID.
    pub id: String,
    pub name: String,
    /// One Kopi excludes out of the box, such as a password manager.
    pub is_default: bool,
}

/// A permission Kopi needs from the OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    /// Pasting into other apps. Without it a paste only copies.
    Accessibility,
    /// Reading copies in the background (macOS 15.4+). Without it nothing is captured.
    PasteFromOtherApps,
}

impl From<Permission> for kopi_platform::Permission {
    fn from(permission: Permission) -> Self {
        match permission {
            Permission::Accessibility => Self::Accessibility,
            Permission::PasteFromOtherApps => Self::PasteFromOtherApps,
        }
    }
}

/// Whether Kopi may read the clipboard in the background, from "Paste from Other Apps".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PasteAccess {
    /// Always allowed, or the OS doesn't ask (before macOS 15.4, or not enforced).
    Allowed,
    /// The user hasn't chosen. Each copy would ask.
    NotAsked,
    /// Each copy asks.
    Ask,
    Denied,
}

#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    pub accessibility: bool,
    pub paste_from_other_apps: PasteAccess,
}

/// The shortcut, the kinds to capture, the theme and automatic updates.
#[tauri::command]
#[specta::specta]
pub async fn get_settings(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<Settings, String> {
    let store = Arc::clone(&store);
    let (kinds, theme, auto_update) =
        blocking(move || Ok((store.capture_kinds()?, store.theme()?, store.auto_update()?)))
            .await?;
    Ok(Settings {
        shortcut: hotkeys::current(&app),
        capture_kinds: kinds.iter().map(Into::into).collect(),
        theme: theme.into(),
        auto_update,
    })
}

/// Switches the shortcut that opens the panel and saves it. Fails, keeping the old one, if the
/// new one is taken. Returns the new one as the plugin writes it.
#[tauri::command]
#[specta::specta]
pub async fn set_shortcut(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    shortcut: String,
) -> Result<String, String> {
    let store = Arc::clone(&store);
    tauri::async_runtime::spawn_blocking(move || hotkeys::change(&app, &store, &shortcut))
        .await
        .map_err(|e| e.to_string())?
}

/// Turns the shortcut off while Settings records a new one, and back on after.
#[tauri::command]
#[specta::specta]
pub async fn suspend_shortcut(app: AppHandle, suspended: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || hotkeys::suspend(&app, suspended))
        .await
        .map_err(|e| e.to_string())
}

/// Saves the kinds to capture. The next copy uses them.
#[tauri::command]
#[specta::specta]
pub async fn set_capture_kinds(
    store: State<'_, Arc<Store>>,
    filter: State<'_, PrivacyFilter>,
    kinds: Vec<ClipKind>,
) -> Result<(), String> {
    let kinds: KindSet = kinds.into_iter().map(storage::ClipKind::from).collect();
    let store = Arc::clone(&store);
    blocking(move || store.set_capture_kinds(kinds)).await?;
    filter.update(|settings| settings.kinds = kinds);
    Ok(())
}

/// Saves the theme and applies it to every window. Sends `theme:changed`.
#[tauri::command]
#[specta::specta]
pub async fn set_theme(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    theme: Theme,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    blocking(move || store.set_theme(theme.into())).await?;
    app.set_theme(theme.into());
    let _ = ThemeChanged { theme }.emit(&app);
    Ok(())
}

/// Turns automatic update checks on or off. The next scheduled check follows it.
#[tauri::command]
#[specta::specta]
pub async fn set_auto_update(store: State<'_, Arc<Store>>, enabled: bool) -> Result<(), String> {
    let store = Arc::clone(&store);
    blocking(move || store.set_auto_update(enabled)).await
}

/// The apps excluded on this OS, by name.
#[tauri::command]
#[specta::specta]
pub async fn excluded_apps(store: State<'_, Arc<Store>>) -> Result<Vec<ExcludedApp>, String> {
    let store = Arc::clone(&store);
    let apps = blocking(move || store.excluded_apps(capture::PLATFORM)).await?;
    Ok(apps
        .into_iter()
        .map(|app| ExcludedApp {
            id: app.app_id,
            name: app.app_name,
            is_default: app.is_default,
        })
        .collect())
}

/// Stops capturing from an app. The next copy from it is skipped.
#[tauri::command]
#[specta::specta]
pub async fn add_excluded_app(
    store: State<'_, Arc<Store>>,
    filter: State<'_, PrivacyFilter>,
    id: String,
    name: String,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let filter = filter.inner().clone();
    blocking(move || {
        store.add_excluded_app(capture::PLATFORM, &id, &name)?;
        capture::reload(&filter, &store)
    })
    .await
}

/// Captures from an app again.
#[tauri::command]
#[specta::specta]
pub async fn remove_excluded_app(
    store: State<'_, Arc<Store>>,
    filter: State<'_, PrivacyFilter>,
    id: String,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let filter = filter.inner().clone();
    blocking(move || {
        store.remove_excluded_app(capture::PLATFORM, &id)?;
        capture::reload(&filter, &store)
    })
    .await
}

/// The apps running now that have a Dock icon, or on Windows a taskbar button, by name, for
/// choosing one to exclude.
#[tauri::command]
#[specta::specta]
pub async fn running_apps() -> Result<Vec<SourceApp>, String> {
    #[cfg(target_os = "macos")]
    let apps = tauri::async_runtime::spawn_blocking(kopi_platform::macos::running_apps)
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    let apps = tauri::async_runtime::spawn_blocking(kopi_platform::windows::running_apps)
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let apps: Vec<kopi_platform::AppRef> = Vec::new();

    let mut apps: Vec<SourceApp> = apps
        .into_iter()
        .filter_map(|app| {
            Some(SourceApp {
                id: app.bundle_id?,
                name: app.name,
            })
        })
        .collect();
    apps.sort_by_cached_key(|app| app.name.as_deref().unwrap_or(&app.id).to_lowercase());
    apps.dedup_by(|a, b| a.id == b.id);
    Ok(apps)
}

/// Whether Kopi has the permissions it needs.
#[tauri::command]
#[specta::specta]
pub async fn permission_status() -> Result<PermissionStatus, String> {
    tauri::async_runtime::spawn_blocking(permissions::status)
        .await
        .map_err(|e| e.to_string())
}

/// Shows the system's request for a permission, where there is one.
#[tauri::command]
#[specta::specta]
pub fn request_permission(permission: Permission) {
    permissions::request(permission.into());
}

/// Opens the page of System Settings where the user grants a permission.
#[tauri::command]
#[specta::specta]
pub fn open_permission_settings(permission: Permission) {
    permissions::open_settings(permission.into());
}

/// Opens the Settings window.
#[tauri::command]
#[specta::specta]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    settings_window::open(&app).map_err(|e| e.to_string())
}

/// Applies the saved theme to the app's windows, at launch.
pub fn apply_theme(app: &AppHandle) {
    let theme = app.state::<Arc<Store>>().theme().unwrap_or_default();
    app.set_theme(Theme::from(theme).into());
}

#[cfg(target_os = "macos")]
mod permissions {
    use kopi_platform::macos::{MacPasteboard, MacPermissions};
    use kopi_platform::{Pasteboard, PasteboardAccess, Permission, Permissions};

    use super::{PasteAccess, PermissionStatus};

    pub fn status() -> PermissionStatus {
        let access = MacPasteboard::general()
            .access()
            .unwrap_or(PasteboardAccess::Default);
        PermissionStatus {
            accessibility: MacPermissions.is_granted(Permission::Accessibility),
            paste_from_other_apps: match access {
                PasteboardAccess::AlwaysAllow => PasteAccess::Allowed,
                PasteboardAccess::Default => PasteAccess::NotAsked,
                PasteboardAccess::Ask => PasteAccess::Ask,
                PasteboardAccess::AlwaysDeny => PasteAccess::Denied,
            },
        }
    }

    pub fn request(permission: Permission) {
        MacPermissions.request(permission);
    }

    pub fn open_settings(permission: Permission) {
        MacPermissions.open_settings(permission);
    }
}

/// Nothing to grant: Windows lets any app read the clipboard and type into other apps.
#[cfg(not(target_os = "macos"))]
mod permissions {
    use kopi_platform::Permission;

    use super::{PasteAccess, PermissionStatus};

    pub fn status() -> PermissionStatus {
        PermissionStatus {
            accessibility: true,
            paste_from_other_apps: PasteAccess::Allowed,
        }
    }

    pub fn request(_: Permission) {}

    pub fn open_settings(_: Permission) {}
}
