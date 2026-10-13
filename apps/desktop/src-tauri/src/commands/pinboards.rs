//! Pinning clips, and the pinboards: listing, editing and filling them.

use std::sync::Arc;

use kopi_core::storage::{self, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use tauri::{AppHandle, State};
use tauri_specta::Event;

use super::clips::{ClipId, blocking};
use crate::events::{ClipUpdated, PinboardsChanged};

/// A pinboard's id, as a command argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct PinboardId(#[specta(type = Number)] pub i64);

/// A pinboard, for the switcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Pinboard {
    #[specta(type = Number)]
    pub id: i64,
    pub name: String,
    /// `#RRGGBB`, or none for the plain marker.
    pub color: Option<String>,
    /// How many clips it holds.
    #[specta(type = Number)]
    pub clip_count: i64,
}

impl From<storage::Pinboard> for Pinboard {
    fn from(board: storage::Pinboard) -> Self {
        Self {
            id: board.id,
            name: board.name,
            color: board.color,
            clip_count: board.clip_count,
        }
    }
}

/// Pins or unpins a clip, and sends `clip:updated`.
#[tauri::command]
#[specta::specta]
pub async fn set_pinned(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: ClipId,
    pinned: bool,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    if blocking(move || store.set_pinned(id.0, pinned)).await? {
        let _ = ClipUpdated { id: id.0 }.emit(&app);
    }
    Ok(())
}

/// Every pinboard, in the user's order.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_list(store: State<'_, Arc<Store>>) -> Result<Vec<Pinboard>, String> {
    let store = Arc::clone(&store);
    let boards = blocking(move || store.pinboards()).await?;
    Ok(boards.into_iter().map(Into::into).collect())
}

/// Adds a pinboard after the others.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_create(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    name: String,
    color: Option<String>,
) -> Result<Pinboard, String> {
    let store = Arc::clone(&store);
    let board = blocking(move || store.create_pinboard(&name, color.as_deref())).await?;
    changed(&app);
    Ok(board.into())
}

/// Renames a pinboard.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_rename(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: PinboardId,
    name: String,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    if !blocking(move || store.rename_pinboard(id.0, &name)).await? {
        return Err(GONE.into());
    }
    changed(&app);
    Ok(())
}

/// Sets a pinboard's color, `#RRGGBB`, or clears it.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_recolor(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: PinboardId,
    color: Option<String>,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    if !blocking(move || store.recolor_pinboard(id.0, color.as_deref())).await? {
        return Err(GONE.into());
    }
    changed(&app);
    Ok(())
}

/// Puts the pinboards in this order. Any left out keep their order after them.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_reorder(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    ids: Vec<PinboardId>,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let ids: Vec<i64> = ids.into_iter().map(|id| id.0).collect();
    blocking(move || store.reorder_pinboards(&ids)).await?;
    changed(&app);
    Ok(())
}

/// Deletes a pinboard. Its clips stay in the history.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_delete(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: PinboardId,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    if blocking(move || store.delete_pinboard(id.0)).await? {
        changed(&app);
    }
    Ok(())
}

/// Puts clips at the top of a pinboard. Clips already on it are skipped.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_add_clips(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: PinboardId,
    clip_ids: Vec<ClipId>,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let clip_ids: Vec<i64> = clip_ids.into_iter().map(|id| id.0).collect();
    let added = blocking(move || store.add_to_pinboard(id.0, &clip_ids)).await?;
    if added.ok_or(GONE)? > 0 {
        changed(&app);
    }
    Ok(())
}

/// Takes clips off a pinboard. They stay in the history.
#[tauri::command]
#[specta::specta]
pub async fn pinboard_remove_clips(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: PinboardId,
    clip_ids: Vec<ClipId>,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let clip_ids: Vec<i64> = clip_ids.into_iter().map(|id| id.0).collect();
    if blocking(move || store.remove_from_pinboard(id.0, &clip_ids)).await? > 0 {
        changed(&app);
    }
    Ok(())
}

/// The ids of the pinboards a clip is on, for its menu.
#[tauri::command]
#[specta::specta]
pub async fn clip_pinboards(
    store: State<'_, Arc<Store>>,
    id: ClipId,
) -> Result<Vec<PinboardId>, String> {
    let store = Arc::clone(&store);
    let ids = blocking(move || store.clip_pinboards(id.0)).await?;
    Ok(ids.into_iter().map(PinboardId).collect())
}

/// The error when a pinboard was deleted while the panel still showed it.
const GONE: &str = "That pinboard was deleted.";

fn changed(app: &AppHandle) {
    let _ = PinboardsChanged.emit(app);
}
