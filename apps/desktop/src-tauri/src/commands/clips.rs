//! One clip at a time: the preview, deleting, and the apps for the filter.

use std::sync::Arc;

use kopi_core::search;
use kopi_core::storage::{self, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use tauri::{AppHandle, State};
use tauri_specta::Event;

use super::search::{ClipKind, ClipMetadata};
use crate::events::ClipsRemoved;

/// How many apps the filter lists.
const MAX_SOURCE_APPS: u32 = 100;

/// Everything the preview shows about a clip.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipDetail {
    #[specta(type = Number)]
    pub id: i64,
    pub kind: ClipKind,
    /// The clip's text, cut to 64 KiB. For an image, the text read from it, if any.
    pub text: Option<String>,
    /// An image whose text Kopi hasn't read yet. The preview says it's looking.
    pub reading_text: bool,
    /// Names the image, for `kopi://…/image/<hash>`.
    pub content_hash: String,
    pub source_app_name: Option<String>,
    pub source_app_id: Option<String>,
    /// Unix milliseconds.
    #[specta(type = Number)]
    pub created_at: i64,
    #[specta(type = Number)]
    pub last_used_at: i64,
    /// How many times it was copied or pasted.
    #[specta(type = Number)]
    pub use_count: i64,
    /// All its stored formats together.
    #[specta(type = Number)]
    pub size_bytes: i64,
    pub pinned: bool,
    pub metadata: Option<ClipMetadata>,
    /// The paired computer it was last copied on. None for this computer.
    pub copied_on: Option<String>,
}

impl ClipDetail {
    /// `reading` says whether Kopi reads text in images here: it's available and turned on.
    fn new(clip: storage::Clip, reading: bool) -> Self {
        Self {
            id: clip.id,
            reading_text: reading
                && clip.kind == storage::ClipKind::Image
                && clip.ocr_version.is_none(),
            kind: clip.kind.into(),
            text: clip.plain_text,
            content_hash: clip.content_hash,
            source_app_name: clip.source_app_name,
            source_app_id: clip.source_app_id,
            created_at: clip.created_at,
            last_used_at: clip.last_used_at,
            use_count: clip.use_count,
            size_bytes: clip.size_bytes,
            pinned: clip.pinned,
            metadata: ClipMetadata::parse(clip.metadata),
            copied_on: clip.copied_on,
        }
    }
}

/// A clip's id, as a command argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(transparent)]
pub struct ClipId(#[specta(type = Number)] pub i64);

/// An app that clips came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct SourceApp {
    /// A macOS bundle ID.
    pub id: String,
    pub name: Option<String>,
}

/// One clip, for the preview. None if it was deleted.
#[tauri::command]
#[specta::specta]
pub async fn get_clip(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: ClipId,
) -> Result<Option<ClipDetail>, String> {
    let store = Arc::clone(&store);
    let available = crate::ocr::available(&app);
    blocking(move || {
        let Some(clip) = store.get_clip(id.0)? else {
            return Ok(None);
        };
        let reading = available && store.image_text_enabled()?;
        Ok(Some(ClipDetail::new(clip, reading)))
    })
    .await
}

/// Deletes clips, and sends `clip:removed` with the ids that were found.
#[tauri::command]
#[specta::specta]
pub async fn delete_clips(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    ids: Vec<ClipId>,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let ids: Vec<i64> = ids.into_iter().map(|id| id.0).collect();
    let deleted = blocking(move || store.delete_clips(&ids)).await?;
    if !deleted.is_empty() {
        let _ = ClipsRemoved { ids: deleted }.emit(&app);
    }
    Ok(())
}

/// The apps in the history, most recent first, for the app filter.
#[tauri::command]
#[specta::specta]
pub async fn source_apps(store: State<'_, Arc<Store>>) -> Result<Vec<SourceApp>, String> {
    let store = Arc::clone(&store);
    let apps = blocking(move || search::source_apps(&store, MAX_SOURCE_APPS)).await?;
    Ok(apps
        .into_iter()
        .map(|app| SourceApp {
            id: app.id,
            name: app.name,
        })
        .collect())
}

/// Runs a store call on a blocking thread, so it doesn't hold up IPC.
pub(super) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> storage::StorageResult<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
