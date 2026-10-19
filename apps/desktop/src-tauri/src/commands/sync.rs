//! Settings → Sync: turning sync on and off, this computer's name, what to sync, the computers
//! in the group, and pairing. See `docs/SYNC_DESIGN.md`.

use std::sync::Arc;

use kopi_core::privacy::KindSet;
use kopi_core::storage::{self, Platform, Store, TOMBSTONE_DAYS, now_ms};
use kopi_sync::{Identity, Problem};
use serde::Serialize;
use specta::Type;
use specta_typescript::Number;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::Os;
use super::clips::blocking;
use super::search::ClipKind;
use crate::events::SyncChanged;
use crate::sync::{self, SyncState};

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub enabled: bool,
    /// What the other computers call this one.
    pub device_name: String,
    /// The kinds sent and taken. Never files.
    pub kinds: Vec<ClipKind>,
    /// The other computers in the group, by name.
    pub devices: Vec<SyncDevice>,
    pub problem: Option<SyncProblem>,
    /// The code and the computers nearby, while the pairing sheet is open.
    pub pairing: Option<Pairing>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncDevice {
    pub id: String,
    pub name: String,
    pub os: Os,
    /// A session with it is running now.
    pub connected: bool,
    /// It runs a newer Kopi, which this one needs to sync with it.
    pub needs_update: bool,
    /// When it last synced with this computer, in Unix milliseconds.
    #[specta(type = Option<Number>)]
    pub last_seen_at: Option<i64>,
    /// It hasn't synced for longer than deletes are remembered, so it may bring deleted clips
    /// back. Removing it and pairing again is safer.
    pub stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SyncProblem {
    /// macOS's Local Network permission is off for Kopi.
    LocalNetworkDenied,
    /// Kopi couldn't advertise or browse on the network.
    Discovery,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Pairing {
    /// 6 digits.
    pub code: String,
    #[specta(type = Number)]
    pub expires_at: i64,
    pub nearby: Vec<NearbyComputer>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct NearbyComputer {
    pub id: String,
    pub name: String,
}

/// Everything Settings → Sync shows.
#[tauri::command]
#[specta::specta]
pub async fn sync_status(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<SyncStatus, String> {
    let status = app
        .state::<SyncState>()
        .with(async |service| service.status())
        .await;
    let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let store = Arc::clone(&store);
    let (enabled, kinds, devices, device_name, self_id) = blocking(move || {
        Ok((
            store.sync_enabled()?,
            store.sync_kinds()?,
            store.sync_devices()?,
            sync::device_name(&store),
            Identity::existing(&data_dir).map(|identity| identity.device_id),
        ))
    })
    .await?;
    let stale_before = now_ms() - i64::from(TOMBSTONE_DAYS) * 24 * 60 * 60 * 1000;
    let devices = devices
        .into_iter()
        .filter(|d| !d.removed && Some(&d.device_id) != self_id.as_ref())
        .map(|d| SyncDevice {
            connected: status
                .as_ref()
                .is_some_and(|s| s.connected.contains(&d.device_id)),
            needs_update: status
                .as_ref()
                .is_some_and(|s| s.needs_update.contains(&d.device_id)),
            stale: d.last_seen_at.is_some_and(|t| t < stale_before),
            id: d.device_id,
            name: d.name,
            os: match d.os {
                Platform::MacOs => Os::Macos,
                Platform::Windows => Os::Windows,
            },
            last_seen_at: d.last_seen_at,
        })
        .collect();
    let status = status.unwrap_or_default();
    Ok(SyncStatus {
        enabled,
        device_name,
        kinds: kinds.iter().map(Into::into).collect(),
        devices,
        problem: status.problem.map(|p| match p {
            Problem::LocalNetworkDenied => SyncProblem::LocalNetworkDenied,
            Problem::Discovery(_) => SyncProblem::Discovery,
        }),
        pairing: status.pairing.map(|p| Pairing {
            code: p.code,
            expires_at: p.expires_at,
            nearby: p
                .nearby
                .into_iter()
                .map(|n| NearbyComputer {
                    id: n.id,
                    name: n.name,
                })
                .collect(),
        }),
    })
}

/// Turns sync on or off. Turning it on makes this computer's identity the first time, and on
/// macOS is when the Local Network permission is asked for.
#[tauri::command]
#[specta::specta]
pub async fn set_sync_enabled(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    enabled: bool,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    blocking(move || store.set_sync_enabled(enabled)).await?;
    if enabled {
        sync::start(&app).await?;
    } else {
        sync::stop(&app).await;
    }
    Ok(())
}

/// Renames this computer. Returns the name as saved, trimmed.
#[tauri::command]
#[specta::specta]
pub async fn set_sync_device_name(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    name: String,
) -> Result<String, String> {
    let store = Arc::clone(&store);
    let name = blocking(move || store.set_sync_device_name(&name)).await?;
    if let Some(result) = app
        .state::<SyncState>()
        .with(async |service| service.rename(&name).await)
        .await
    {
        result.map_err(|e| e.to_string())?;
    }
    let _ = SyncChanged.emit(&app);
    Ok(name)
}

/// The kinds to send and take. Files are never synced.
#[tauri::command]
#[specta::specta]
pub async fn set_sync_kinds(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    kinds: Vec<ClipKind>,
) -> Result<(), String> {
    let kinds: KindSet = kinds.into_iter().map(storage::ClipKind::from).collect();
    let store = Arc::clone(&store);
    blocking(move || store.set_sync_kinds(kinds)).await?;
    let _ = SyncChanged.emit(&app);
    Ok(())
}

/// Removes a computer from the group, on every computer in it. It keeps what it already has.
#[tauri::command]
#[specta::specta]
pub async fn remove_sync_device(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: String,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let device = id.clone();
    blocking(move || store.remove_device(&device)).await?;
    app.state::<SyncState>()
        .with(async |service| service.disconnect(&id))
        .await;
    let _ = SyncChanged.emit(&app);
    Ok(())
}

/// Opens pairing: a code to show, and this computer listed to others pairing nearby.
#[tauri::command]
#[specta::specta]
pub async fn start_pairing(app: AppHandle) -> Result<(), String> {
    app.state::<SyncState>()
        .with(async |service| service.start_pairing())
        .await
        .ok_or("Sync is off.")?
        .map_err(|e| e.to_string())
}

/// Closes pairing. The code stops working.
#[tauri::command]
#[specta::specta]
pub async fn stop_pairing(app: AppHandle) -> Result<(), String> {
    app.state::<SyncState>()
        .with(async |service| service.stop_pairing())
        .await
        .unwrap_or(Ok(()))
        .map_err(|e| e.to_string())
}

/// Pairs with a computer nearby, with the code it shows. Returns its name.
#[tauri::command]
#[specta::specta]
pub async fn pair_with(app: AppHandle, id: String, code: String) -> Result<String, String> {
    app.state::<SyncState>()
        .with(async |service| service.pair(&id, &code).await)
        .await
        .ok_or("Sync is off.")?
        .map_err(|e| match e {
            kopi_sync::Error::WrongCode => "That code didn't match. Check the code on the other computer, which has a new one now.".to_owned(),
            kopi_sync::Error::BadCode => "Enter the 6-digit code shown on the other computer.".to_owned(),
            kopi_sync::Error::NotPairing => "That computer isn't ready to pair any more. Open Add a computer on it again.".to_owned(),
            kopi_sync::Error::NeedsUpdate => "That computer has a newer Kopi. Update this one first.".to_owned(),
            e => format!("Couldn't pair. {e}"),
        })
}

/// Opens System Settings at Privacy & Security → Local Network, where Kopi can be allowed.
/// Does nothing outside macOS.
#[tauri::command]
#[specta::specta]
pub fn open_local_network_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork")
            .spawn();
    }
}
