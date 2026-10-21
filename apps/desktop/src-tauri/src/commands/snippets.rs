//! Snippets: listing, searching and editing them. Pasting is in `paste.rs`.

use std::sync::Arc;

use kopi_core::storage::{self, SnippetEdit, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use tauri::{AppHandle, State};
use tauri_specta::Event;

use super::clips::blocking;
use crate::events::SnippetsChanged;

/// A snippet's id, as a command argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct SnippetId(#[specta(type = Number)] pub i64);

/// A snippet, with its text as typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    #[specta(type = Number)]
    pub id: i64,
    pub name: String,
    pub keyword: Option<String>,
    /// Placeholders and all. The panel fills them in when it pastes.
    pub body: String,
    /// Unix milliseconds.
    #[specta(type = Number)]
    pub created_at: i64,
    #[specta(type = Number)]
    pub updated_at: i64,
    /// When it was last pasted or copied on this computer.
    #[specta(type = Option<Number>)]
    pub last_used_at: Option<i64>,
    /// How many times it was pasted or copied on this computer.
    #[specta(type = Number)]
    pub use_count: i64,
}

impl From<storage::Snippet> for Snippet {
    fn from(s: storage::Snippet) -> Self {
        Self {
            id: s.id,
            name: s.name,
            keyword: s.keyword,
            body: s.body,
            created_at: s.created_at,
            updated_at: s.updated_at,
            last_used_at: s.last_used_at,
            use_count: s.use_count,
        }
    }
}

/// What the editor sends.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Type)]
pub struct SnippetFields {
    pub name: String,
    /// None or empty for no keyword.
    pub keyword: Option<String>,
    pub body: String,
}

impl SnippetFields {
    fn edit(&self) -> SnippetEdit<'_> {
        SnippetEdit {
            name: &self.name,
            keyword: self.keyword.as_deref(),
            body: &self.body,
        }
    }
}

/// The snippets for the search box, an exact keyword first. Empty text lists them all, in the
/// user's order.
#[tauri::command]
#[specta::specta]
pub async fn snippet_search(
    store: State<'_, Arc<Store>>,
    text: String,
) -> Result<Vec<Snippet>, String> {
    let store = Arc::clone(&store);
    let found = blocking(move || store.find_snippets(&text)).await?;
    Ok(found.into_iter().map(Into::into).collect())
}

/// The snippets whose keyword is exactly `text`, which the History tab shows first.
#[tauri::command]
#[specta::specta]
pub async fn snippet_keyword(
    store: State<'_, Arc<Store>>,
    text: String,
) -> Result<Vec<Snippet>, String> {
    let store = Arc::clone(&store);
    let found = blocking(move || store.snippets_with_keyword(&text)).await?;
    Ok(found.into_iter().map(Into::into).collect())
}

/// One snippet. None if it was deleted.
#[tauri::command]
#[specta::specta]
pub async fn snippet_get(
    store: State<'_, Arc<Store>>,
    id: SnippetId,
) -> Result<Option<Snippet>, String> {
    let store = Arc::clone(&store);
    let snippet = blocking(move || store.snippet(id.0)).await?;
    Ok(snippet.map(Into::into))
}

/// Adds a snippet at the top of the list.
#[tauri::command]
#[specta::specta]
pub async fn snippet_create(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    fields: SnippetFields,
) -> Result<Snippet, String> {
    let store = Arc::clone(&store);
    let snippet = blocking(move || store.create_snippet(fields.edit())).await?;
    changed(&app);
    Ok(snippet.into())
}

/// Saves the editor's fields.
#[tauri::command]
#[specta::specta]
pub async fn snippet_update(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: SnippetId,
    fields: SnippetFields,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    if !blocking(move || store.update_snippet(id.0, fields.edit())).await? {
        return Err(GONE.into());
    }
    changed(&app);
    Ok(())
}

/// Puts the snippets in this order. Any left out keep their order after them.
#[tauri::command]
#[specta::specta]
pub async fn snippet_reorder(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    ids: Vec<SnippetId>,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let ids: Vec<i64> = ids.into_iter().map(|id| id.0).collect();
    blocking(move || store.reorder_snippets(&ids)).await?;
    changed(&app);
    Ok(())
}

/// Deletes a snippet.
#[tauri::command]
#[specta::specta]
pub async fn snippet_delete(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    id: SnippetId,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    if blocking(move || store.delete_snippet(id.0)).await? {
        changed(&app);
    }
    Ok(())
}

/// The error when a snippet was deleted while the panel still showed it.
pub(super) const GONE: &str = "That snippet was deleted.";

pub(super) fn changed(app: &AppHandle) {
    let _ = SnippetsChanged.emit(app);
}
