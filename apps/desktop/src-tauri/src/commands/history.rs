//! How much history Kopi keeps, and clearing it.

use std::sync::Arc;

use kopi_core::retention::{Limits, Retention};
use kopi_core::storage::{Store, now_ms};
use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::clips::blocking;
use crate::events::ClipsRemoved;

/// The history limits, each unlimited when unset. Pinned clips and clips on a pinboard are never
/// removed by them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLimits {
    /// The most clips to keep, not counting pinned ones and those on a pinboard.
    #[specta(type = Option<Number>)]
    pub max_items: Option<u64>,
    /// Clips not used for this many days are removed.
    pub max_age_days: Option<u32>,
    /// The most space all clips may take, pinned or not.
    #[specta(type = Option<Number>)]
    pub max_total_bytes: Option<u64>,
}

impl From<Limits> for HistoryLimits {
    fn from(limits: Limits) -> Self {
        Self {
            max_items: limits.max_items,
            max_age_days: limits.max_age_days,
            max_total_bytes: limits.max_total_bytes,
        }
    }
}

impl From<HistoryLimits> for Limits {
    fn from(limits: HistoryLimits) -> Self {
        Self {
            max_items: limits.max_items,
            max_age_days: limits.max_age_days,
            max_total_bytes: limits.max_total_bytes,
        }
    }
}

/// Which clips "Clear history" removes, by when they were last used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ClearRange {
    LastHour,
    LastDay,
    All,
}

impl ClearRange {
    /// The earliest `last_used_at` removed, or none for everything.
    fn since(self, now: i64) -> Option<i64> {
        const HOUR: i64 = 60 * 60 * 1000;
        match self {
            Self::LastHour => Some(now - HOUR),
            Self::LastDay => Some(now - 24 * HOUR),
            Self::All => None,
        }
    }
}

/// The history limits in the settings.
#[tauri::command]
#[specta::specta]
pub async fn history_limits(store: State<'_, Arc<Store>>) -> Result<HistoryLimits, String> {
    let store = Arc::clone(&store);
    blocking(move || store.retention_limits())
        .await
        .map(Into::into)
}

/// Saves the history limits. Lowering one applies it straight away.
#[tauri::command]
#[specta::specta]
pub async fn set_history_limits(
    store: State<'_, Arc<Store>>,
    retention: State<'_, Retention>,
    limits: HistoryLimits,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let limits = Limits::from(limits);
    let stricter = blocking(move || {
        let old = store.retention_limits()?;
        store.set_retention_limits(&limits)?;
        Ok(limits.is_stricter_than(&old))
    })
    .await?;
    if stricter {
        retention.run_now();
    }
    Ok(())
}

/// Deletes the clips in `range`, keeping pinned clips and those on a pinboard when
/// `keep_pinned` is set, and compacts the database. Sends `clip:removed`. Returns how many
/// clips were deleted.
#[tauri::command]
#[specta::specta]
pub async fn clear_history(
    app: AppHandle,
    range: ClearRange,
    keep_pinned: bool,
) -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(move || clear(&app, range, keep_pinned))
        .await
        .map_err(|e| e.to_string())?
}

/// [`clear_history`] on the calling thread, which it blocks. The tray menu uses it too.
pub fn clear(app: &AppHandle, range: ClearRange, keep_pinned: bool) -> Result<u32, String> {
    let store = app.state::<Arc<Store>>();
    let ids = store
        .clear_history(range.since(now_ms()), keep_pinned)
        .map_err(|e| e.to_string())?;
    let count = ids.len() as u32;
    if !ids.is_empty() {
        let _ = ClipsRemoved { ids }.emit(app);
    }
    Ok(count)
}
