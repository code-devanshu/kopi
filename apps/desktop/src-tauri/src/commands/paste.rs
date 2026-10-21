//! `paste_clip` and `copy_clip`: Return, ⌥Return, ⌘1–9 and ⌘C in the panel. And the same
//! for snippets, whose text the panel fills in first.

use std::sync::Arc;

use kopi_core::paste::{Content, PasteEngine, Pasted};
use kopi_core::storage::Store;
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::clips::{ClipId, blocking};
use super::snippets::{self, SnippetId};
use crate::events::ClipUpdated;
use crate::panel;

/// How a paste ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum PasteOutcome {
    /// Pasted into the app in front. The panel is hidden.
    Pasted,
    /// Kopi may not post keys yet, so the clip is on the clipboard for the user to paste. The
    /// panel is still showing.
    Copied,
}

/// Pastes a clip into the app in front, or only copies it while Accessibility isn't granted.
/// `plain` pastes only its plain text.
#[tauri::command]
#[specta::specta]
pub async fn paste_clip(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: ClipId,
    plain: bool,
) -> Result<PasteOutcome, String> {
    let engine = engine(&app)?;
    let store = Arc::clone(&store);
    let handle = app.clone();
    let pasted =
        run(move || engine.paste(&store, id.0, content(plain), || panel::hide_now(&handle)))
            .await?;
    let _ = ClipUpdated { id: id.0 }.emit(&app);
    Ok(match pasted {
        Pasted::Pasted => PasteOutcome::Pasted,
        Pasted::Copied => PasteOutcome::Copied,
    })
}

/// Puts a clip on the clipboard without pasting it. `plain` copies only its plain text.
#[tauri::command]
#[specta::specta]
pub async fn copy_clip(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: ClipId,
    plain: bool,
) -> Result<(), String> {
    let engine = engine(&app)?;
    let store = Arc::clone(&store);
    run(move || engine.copy(&store, id.0, content(plain))).await?;
    let _ = ClipUpdated { id: id.0 }.emit(&app);
    Ok(())
}

/// Pastes a snippet's filled-in `text` into the app in front as plain text, then moves the
/// cursor `cursor_back` characters left, for `{cursor}`. Without Accessibility it only copies.
#[tauri::command]
#[specta::specta]
pub async fn paste_snippet(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: SnippetId,
    text: String,
    cursor_back: u32,
) -> Result<PasteOutcome, String> {
    let engine = engine(&app)?;
    let store = Arc::clone(&store);
    let handle = app.clone();
    let pasted = run(move || {
        engine.paste_snippet(&store, id.0, &text, cursor_back as usize, || {
            panel::hide_now(&handle)
        })
    })
    .await
    .map_err(gone)?;
    Ok(match pasted {
        Pasted::Pasted => PasteOutcome::Pasted,
        Pasted::Copied => PasteOutcome::Copied,
    })
}

/// Puts a snippet's filled-in `text` on the clipboard without pasting it.
#[tauri::command]
#[specta::specta]
pub async fn copy_snippet(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: SnippetId,
    text: String,
) -> Result<(), String> {
    let engine = engine(&app)?;
    let store = Arc::clone(&store);
    run(move || engine.copy_snippet(&store, id.0, &text))
        .await
        .map_err(gone)
}

/// The newest text in the history, in full, for `{clipboard}`. Never the live clipboard, so
/// it can't be something Kopi chose not to keep.
#[tauri::command]
#[specta::specta]
pub async fn clipboard_text(store: State<'_, Arc<Store>>) -> Result<Option<String>, String> {
    let store = Arc::clone(&store);
    blocking(move || {
        kopi_core::paste::newest_text(&store).map_err(|e| match e {
            kopi_core::paste::PasteError::Storage(e) => e,
            other => kopi_core::storage::StorageError::Corrupt(other.to_string()),
        })
    })
    .await
}

/// "no snippet 4" reads as the deleted snippet it is.
fn gone(error: String) -> String {
    if error.starts_with("no snippet") {
        snippets::GONE.to_owned()
    } else {
        error
    }
}

/// Sets up the paste engine.
#[cfg(target_os = "macos")]
pub fn manage_engine(app: &AppHandle) {
    use kopi_platform::macos::{MacKeySender, MacPasteboard, MacPermissions};

    app.manage(Arc::new(PasteEngine::new(
        Box::new(MacPasteboard::general()),
        Box::new(MacKeySender),
        Box::new(MacPermissions),
        app.config().identifier.clone(),
    )));
}

/// Sets up the paste engine.
#[cfg(target_os = "windows")]
pub fn manage_engine(app: &AppHandle) {
    use kopi_platform::windows::{WinKeySender, WinPasteboard, WinPermissions};

    app.manage(Arc::new(PasteEngine::new(
        Box::new(WinPasteboard::general()),
        Box::new(WinKeySender),
        Box::new(WinPermissions),
        app.config().identifier.clone(),
    )));
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn manage_engine(_app: &AppHandle) {}

fn engine(app: &AppHandle) -> Result<Arc<PasteEngine>, String> {
    app.try_state::<Arc<PasteEngine>>()
        .map(|engine| Arc::clone(&engine))
        .ok_or_else(|| "pasting isn't supported on this OS yet".to_owned())
}

fn content(plain: bool) -> Content {
    if plain {
        Content::PlainText
    } else {
        Content::All
    }
}

/// Runs on a blocking thread: the store reads files, and the pasteboard waits for the main
/// thread.
async fn run<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, kopi_core::paste::PasteError> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
