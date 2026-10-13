//! Events the UI listens for. `bindings.ts` has a typed `listen` for each.

use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use tauri_specta::Event;

/// A copy added a new clip.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "clip:added")]
pub struct ClipAdded {
    #[specta(type = Number)]
    pub id: i64,
}

/// A clip changed: copied again, so it moved to the top.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "clip:updated")]
pub struct ClipUpdated {
    #[specta(type = Number)]
    pub id: i64,
}

/// Clips were deleted.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "clip:removed")]
pub struct ClipsRemoved {
    #[specta(type = Vec<Number>)]
    pub ids: Vec<i64>,
}

/// Pinboards were added, changed, reordered or deleted, or clips were added to or taken off
/// one. The panel reloads the switcher and the list.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "pinboards:changed")]
pub struct PinboardsChanged;

/// An image clip's thumbnail was written, so `kopi://…/thumb/<hash>` now loads.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "thumbnail:ready")]
pub struct ThumbnailReady {
    pub content_hash: String,
}

/// The panel was shown. The search box takes focus.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "panel:shown")]
pub struct PanelShown;

/// The panel was hidden.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "panel:hidden")]
pub struct PanelHidden;
