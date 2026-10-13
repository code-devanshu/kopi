//! `search_clips`: the panel's history list and search box.

use std::sync::Arc;

use kopi_core::search;
use kopi_core::storage::{self, Store};
use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Number;
use tauri::State;

/// What a clip is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ClipKind {
    Text,
    RichText,
    Link,
    Image,
    File,
    Color,
    Code,
}

impl From<ClipKind> for storage::ClipKind {
    fn from(kind: ClipKind) -> Self {
        match kind {
            ClipKind::Text => Self::Text,
            ClipKind::RichText => Self::RichText,
            ClipKind::Link => Self::Link,
            ClipKind::Image => Self::Image,
            ClipKind::File => Self::File,
            ClipKind::Color => Self::Color,
            ClipKind::Code => Self::Code,
        }
    }
}

impl From<storage::ClipKind> for ClipKind {
    fn from(kind: storage::ClipKind) -> Self {
        match kind {
            storage::ClipKind::Text => Self::Text,
            storage::ClipKind::RichText => Self::RichText,
            storage::ClipKind::Link => Self::Link,
            storage::ClipKind::Image => Self::Image,
            storage::ClipKind::File => Self::File,
            storage::ClipKind::Color => Self::Color,
            storage::ClipKind::Code => Self::Code,
        }
    }
}

/// What to search for. Empty text lists the most recent clips.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    /// Words to find anywhere in a clip's text or its app's name. All of them must match.
    pub text: String,
    pub kind: Option<ClipKind>,
    pub source_app_id: Option<String>,
    #[specta(type = Option<Number>)]
    pub pinboard_id: Option<i64>,
    pub offset: u32,
    /// At most 200. Zero means 50.
    pub limit: u32,
}

impl From<SearchQuery> for search::SearchQuery {
    fn from(q: SearchQuery) -> Self {
        Self {
            text: q.text,
            kind: q.kind.map(Into::into),
            source_app_id: q.source_app_id,
            pinboard_id: q.pinboard_id,
            offset: q.offset,
            limit: q.limit,
        }
    }
}

/// What the classifier found out about a clip. Which fields are set depends on the kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct ClipMetadata {
    /// Image width and height in pixels.
    pub w: Option<u32>,
    pub h: Option<u32>,
    /// Color as red, green, blue and alpha, 0–255.
    pub rgba: Option<Vec<u8>>,
    /// A link's host.
    pub host: Option<String>,
    /// How many files, and their names.
    pub count: Option<u32>,
    pub names: Option<Vec<String>>,
    /// A code clip's language, when one clearly stands out.
    pub lang: Option<String>,
}

/// A run of snippet text. `matched` runs are what the search found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Span {
    pub text: String,
    pub matched: bool,
}

/// One clip in the results.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    #[specta(type = Number)]
    pub id: i64,
    pub kind: ClipKind,
    pub content_hash: String,
    pub source_app_name: Option<String>,
    pub source_app_id: Option<String>,
    /// Unix milliseconds.
    #[specta(type = Number)]
    pub last_used_at: i64,
    pub pinned: bool,
    pub metadata: Option<ClipMetadata>,
    /// The text around the first match, or the start of the text. Empty for images.
    pub snippet: Vec<Span>,
}

impl From<search::SearchHit> for SearchHit {
    fn from(hit: search::SearchHit) -> Self {
        Self {
            id: hit.id,
            kind: hit.kind.into(),
            content_hash: hit.content_hash,
            source_app_name: hit.source_app_name,
            source_app_id: hit.source_app_id,
            last_used_at: hit.last_used_at,
            pinned: hit.pinned,
            // Metadata Kopi wrote itself; anything it can't read is left out.
            metadata: hit.metadata.and_then(|m| serde_json::from_value(m).ok()),
            snippet: hit
                .snippet
                .into_iter()
                .map(|s| Span {
                    text: s.text,
                    matched: s.matched,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub hits: Vec<SearchHit>,
    /// Whether asking again from `offset + hits.length` would find more.
    pub has_more: bool,
}

/// Searches the history, or lists the most recent clips when the text is empty.
#[tauri::command]
#[specta::specta]
pub async fn search_clips(
    store: State<'_, Arc<Store>>,
    query: SearchQuery,
) -> Result<SearchPage, String> {
    let store = Arc::clone(&store);
    let page = tauri::async_runtime::spawn_blocking(move || {
        search::search(&store, &query.into()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(SearchPage {
        hits: page.hits.into_iter().map(Into::into).collect(),
        has_more: page.has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kinds_round_trip() {
        for kind in storage::ClipKind::ALL {
            let ours = ClipKind::from(kind);
            assert_eq!(storage::ClipKind::from(ours), kind);
            // The same names as the database.
            assert_eq!(serde_json::to_value(ours).unwrap(), json!(kind.as_str()));
        }
    }

    #[test]
    fn metadata_from_every_kind() {
        let cases = [
            (
                json!({ "w": 4, "h": 3 }),
                ClipMetadata {
                    w: Some(4),
                    h: Some(3),
                    ..ClipMetadata::default()
                },
            ),
            (
                json!({ "rgba": [30, 144, 255, 128] }),
                ClipMetadata {
                    rgba: Some(vec![30, 144, 255, 128]),
                    ..ClipMetadata::default()
                },
            ),
            (
                json!({ "host": "example.com" }),
                ClipMetadata {
                    host: Some("example.com".into()),
                    ..ClipMetadata::default()
                },
            ),
            (
                json!({ "count": 2, "names": ["a.txt", "b"] }),
                ClipMetadata {
                    count: Some(2),
                    names: Some(vec!["a.txt".into(), "b".into()]),
                    ..ClipMetadata::default()
                },
            ),
            (
                json!({ "lang": "rust" }),
                ClipMetadata {
                    lang: Some("rust".into()),
                    ..ClipMetadata::default()
                },
            ),
        ];
        for (value, want) in cases {
            let got: ClipMetadata = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(got, want, "{value}");
        }
    }

    #[test]
    fn query_fields_are_optional() {
        let query: SearchQuery = serde_json::from_value(json!({ "text": "abc" })).unwrap();
        let query = search::SearchQuery::from(query);
        assert_eq!(
            query,
            search::SearchQuery {
                text: "abc".into(),
                ..search::SearchQuery::default()
            }
        );
        let query: SearchQuery = serde_json::from_value(json!({
            "text": "",
            "kind": "rich_text",
            "sourceAppId": "com.apple.Safari",
            "pinboardId": 7,
            "offset": 50,
            "limit": 25,
        }))
        .unwrap();
        let query = search::SearchQuery::from(query);
        assert_eq!(query.kind, Some(storage::ClipKind::RichText));
        assert_eq!(query.source_app_id.as_deref(), Some("com.apple.Safari"));
        assert_eq!(
            (query.pinboard_id, query.offset, query.limit),
            (Some(7), 50, 25)
        );
    }
}
