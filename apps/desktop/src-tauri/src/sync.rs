//! Sync with the user's other computers: starts and stops `kopi-sync`, and turns what it
//! reports into UI events. While sync is off, nothing runs and no socket is open.

use std::sync::Arc;

use kopi_core::retention::Retention;
use kopi_core::storage::Store;
use kopi_sync::{Config, EventSink, SyncEvent, SyncService};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::events::{
    ClipAdded, ClipUpdated, ClipsRemoved, PinboardsChanged, SyncChanged, SyncPaired,
    SyncPairingFailed,
};

/// The running service, if sync is on.
#[derive(Default)]
pub struct SyncState {
    service: tauri::async_runtime::Mutex<Option<SyncService>>,
}

impl SyncState {
    /// Runs `f` with the service, or returns `None` if sync is off.
    pub async fn with<T>(&self, f: impl AsyncFnOnce(&SyncService) -> T) -> Option<T> {
        let service = self.service.lock().await;
        match service.as_ref() {
            Some(service) => Some(f(service).await),
            None => None,
        }
    }
}

/// Starts sync at launch, if it's on.
pub fn start_if_enabled(app: &AppHandle, store: &Store) {
    app.manage(SyncState::default());
    if store.sync_enabled().unwrap_or(false) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = start(&app).await {
                eprintln!("kopi: sync: couldn't start: {e}");
            }
        });
    }
}

/// Starts the service. Does nothing if it's running.
pub async fn start(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<SyncState>();
    let mut service = state.service.lock().await;
    if service.is_some() {
        return Ok(());
    }
    let store = Arc::clone(&app.state::<Arc<Store>>());
    let config = Config {
        data_dir: app.path().app_local_data_dir().map_err(|e| e.to_string())?,
        app_version: kopi_core::VERSION.to_owned(),
        device_name: device_name(&store),
        discovery: true,
    };
    let handle = app.clone();
    let events: EventSink = Arc::new(move |event| on_event(&handle, event));
    *service = Some(
        SyncService::start(store, config, events)
            .await
            .map_err(|e| e.to_string())?,
    );
    drop(service);
    let _ = SyncChanged.emit(app);
    Ok(())
}

/// Stops the service, if it's running.
pub async fn stop(app: &AppHandle) {
    let state = app.state::<SyncState>();
    let service = state.service.lock().await.take();
    if let Some(service) = service {
        service.stop().await;
    }
    let _ = SyncChanged.emit(app);
}

/// The name this computer goes by: the one the user gave it, or the computer's own.
pub fn device_name(store: &Store) -> String {
    store
        .sync_device_name()
        .ok()
        .flatten()
        .unwrap_or_else(kopi_sync::default_device_name)
}

fn on_event(app: &AppHandle, event: SyncEvent) {
    match event {
        SyncEvent::Applied(applied) => {
            // One event of each kind per batch. The panel reloads once for all of them.
            if let Some(&id) = applied.added.last() {
                let _ = ClipAdded { id }.emit(app);
                app.state::<Retention>().clips_added();
            }
            if let Some(&id) = applied.updated.last() {
                let _ = ClipUpdated { id }.emit(app);
            }
            if !applied.removed.is_empty() {
                let _ = ClipsRemoved {
                    ids: applied.removed,
                }
                .emit(app);
            }
            if applied.pinboards_changed {
                let _ = PinboardsChanged.emit(app);
            }
            if applied.devices_changed {
                let _ = SyncChanged.emit(app);
            }
        }
        SyncEvent::StatusChanged => {
            let _ = SyncChanged.emit(app);
        }
        SyncEvent::Paired { name } => {
            let _ = SyncPaired { name }.emit(app);
        }
        SyncEvent::PairingFailed { reason } => {
            let _ = SyncPairingFailed { reason }.emit(app);
        }
    }
}
