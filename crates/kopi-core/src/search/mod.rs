//! Search over the history, as the panel runs it on every keystroke. See `docs/DATA_MODEL.md`.
//!
//! - **No text:** clips most recently used first, or in the pinboard's own order when filtered
//!   by a pinboard.
//! - **Words of 3 or more characters** go to the trigram index, each as an escaped FTS5 string
//!   literal, and all of them must match. The newest [`MAX_CANDIDATES`] matches are ranked by
//!   bm25's length normalisation blended with recency (see `rank.rs`), so a common fragment
//!   stays within the typing budget.
//! - **Shorter words** can't use trigrams. Next to a longer word, they narrow its matches with
//!   `LIKE`. On their own, they're matched with `LIKE` against the [`SHORT_SCAN`] most recently
//!   used clips.
//!
//! The kind, source app, pinned and pinboard filters apply to every path. Snippets are built only for
//! the page that is returned.

mod query;
mod rank;
mod snippet;

use rusqlite::types::Value;
use rusqlite::{Connection, Row, params_from_iter};

use crate::storage::{ClipKind, StorageError, StorageResult, Store};

pub use snippet::Span;

use query::Terms;
use rank::Candidate;

/// Index matches ranked per search. Older matches past this are left out.
pub const MAX_CANDIDATES: usize = 1000;

/// How many of the most recently used clips a search of only 1–2 character words looks at.
pub const SHORT_SCAN: usize = 5000;

/// The most clips one page can hold.
pub const MAX_PAGE: u32 = 200;

/// The page size when the query asks for none.
pub const DEFAULT_PAGE: u32 = 50;

/// What to search for. `Default` lists the most recent clips.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchQuery {
    /// What was typed in the search box. Words are matched anywhere in the clip's text or its
    /// source app's name, case and accents aside, and all of them must match.
    pub text: String,
    pub kind: Option<ClipKind>,
    /// A macOS bundle ID or Windows app ID, as stored in `clips.source_app_id`.
    pub source_app_id: Option<String>,
    pub pinboard_id: Option<i64>,
    /// Only pinned clips.
    pub pinned: bool,
    pub offset: u32,
    /// At most [`MAX_PAGE`]. Zero means [`DEFAULT_PAGE`].
    pub limit: u32,
}

/// One clip in the results, with what the list row needs.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub id: i64,
    pub kind: ClipKind,
    /// Names the clip's thumbnail, for images.
    pub content_hash: String,
    pub source_app_name: Option<String>,
    pub source_app_id: Option<String>,
    pub last_used_at: i64,
    pub pinned: bool,
    pub metadata: Option<serde_json::Value>,
    /// The text around the first match, with the matches marked, or the start of the text when
    /// there's nothing to mark. Empty for clips without text, such as images.
    pub snippet: Vec<Span>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchPage {
    pub hits: Vec<SearchHit>,
    /// Whether a page at `offset + hits.len()` would have more.
    pub has_more: bool,
}

/// Runs `query` on one of the store's read connections.
pub fn search(store: &Store, query: &SearchQuery) -> StorageResult<SearchPage> {
    let terms = Terms::parse(&query.text);
    let page = Page::of(query);
    store.with_reader(|conn| {
        // One snapshot, so a clip deleted between ranking and reading the page can't fail it.
        let snapshot = conn.unchecked_transaction()?;
        if !terms.long.is_empty() {
            search_index(&snapshot, query, &terms, page)
        } else if !terms.short.is_empty() {
            search_recent(&snapshot, query, &terms, page)
        } else {
            list(&snapshot, query, page)
        }
    })
}

/// An app that clips came from, for the app filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceApp {
    /// A macOS bundle ID or Windows app ID.
    pub id: String,
    /// The name it had on its most recent clip.
    pub name: Option<String>,
}

/// The apps in the history, the one copied from most recently first. At most `limit`.
pub fn source_apps(store: &Store, limit: u32) -> StorageResult<Vec<SourceApp>> {
    store.with_reader(|conn| {
        // SQLite takes a bare column next to max() from the row with the maximum.
        let apps = conn
            .prepare_cached(
                "SELECT source_app_id, source_app_name, max(last_used_at) AS last FROM clips
                 WHERE source_app_id IS NOT NULL
                 GROUP BY source_app_id ORDER BY last DESC LIMIT ?1",
            )?
            .query_map([limit], |row| {
                Ok(SourceApp {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(apps)
    })
}

#[derive(Debug, Clone, Copy)]
struct Page {
    offset: usize,
    limit: usize,
}

impl Page {
    fn of(query: &SearchQuery) -> Self {
        let limit = match query.limit {
            0 => DEFAULT_PAGE,
            n => n.min(MAX_PAGE),
        };
        Self {
            offset: query.offset as usize,
            limit: limit as usize,
        }
    }

    /// Keeps one page of `rows`, which were fetched with one row past the page.
    fn cut<T>(self, mut rows: Vec<T>) -> (Vec<T>, bool) {
        let has_more = rows.len() > self.limit;
        rows.truncate(self.limit);
        (rows, has_more)
    }
}

/// The columns [`hit`] reads, after the snippet source at index 0.
const HIT_COLUMNS: &str = "c.id, c.kind, c.content_hash, c.source_app_name, c.source_app_id,
                           c.last_used_at, c.pinned, c.metadata";

/// A hit from a row of `?, HIT_COLUMNS`, with the snippet already made.
fn hit(row: &Row<'_>, snippet: Vec<Span>) -> rusqlite::Result<(SearchHit, String, Option<String>)> {
    Ok((
        SearchHit {
            id: row.get(1)?,
            kind: ClipKind::Text,
            content_hash: row.get(3)?,
            source_app_name: row.get(4)?,
            source_app_id: row.get(5)?,
            last_used_at: row.get(6)?,
            pinned: row.get(7)?,
            metadata: None,
            snippet,
        },
        row.get(2)?,
        row.get(8)?,
    ))
}

/// Fills in the kind and metadata, which can fail to parse.
fn finish(
    (mut hit, kind, metadata): (SearchHit, String, Option<String>),
) -> StorageResult<SearchHit> {
    hit.kind = kind.parse()?;
    hit.metadata = metadata
        .map(|m| serde_json::from_str(&m))
        .transpose()
        .map_err(|e| StorageError::Corrupt(format!("metadata of clip {}: {e}", hit.id)))?;
    Ok(hit)
}

/// Appends the kind, app and pinned filters on `c`, and the pinboard filter unless the caller
/// joins the pinboard itself.
fn filters(query: &SearchQuery, pinboard: bool, sql: &mut String, params: &mut Vec<Value>) {
    if let Some(kind) = query.kind {
        sql.push_str(" AND c.kind = ?");
        params.push(kind.as_str().to_owned().into());
    }
    if let Some(app) = &query.source_app_id {
        sql.push_str(" AND c.source_app_id = ?");
        params.push(app.clone().into());
    }
    if query.pinned {
        sql.push_str(" AND c.pinned = 1");
    }
    if let (true, Some(id)) = (pinboard, query.pinboard_id) {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM pinboard_items p WHERE p.pinboard_id = ? AND p.clip_id = c.id)",
        );
        params.push(id.into());
    }
}

/// Appends a `LIKE` for each short word, on the text or the app name, as the index matches.
fn like_filters(terms: &Terms, sql: &mut String, params: &mut Vec<Value>) {
    for term in &terms.short {
        sql.push_str(
            " AND (c.plain_text LIKE ? ESCAPE '\\' OR c.source_app_name LIKE ? ESCAPE '\\')",
        );
        let pattern = query::like_pattern(term);
        params.push(pattern.clone().into());
        params.push(pattern.into());
    }
}

/// No text: the most recent clips, or a pinboard in its own order.
fn list(conn: &Connection, query: &SearchQuery, page: Page) -> StorageResult<SearchPage> {
    let mut params = Vec::new();
    let mut sql = format!(
        "SELECT substr(c.plain_text, 1, {}), {HIT_COLUMNS}",
        snippet::PREVIEW_CHARS + 1
    );
    match query.pinboard_id {
        Some(id) => {
            sql.push_str(
                " FROM pinboard_items p JOIN clips c ON c.id = p.clip_id WHERE p.pinboard_id = ?",
            );
            params.push(id.into());
            filters(query, false, &mut sql, &mut params);
            sql.push_str(" ORDER BY p.position, c.id");
        }
        None => {
            sql.push_str(" FROM clips c WHERE 1");
            filters(query, false, &mut sql, &mut params);
            sql.push_str(" ORDER BY c.last_used_at DESC, c.id DESC");
        }
    }
    sql.push_str(" LIMIT ? OFFSET ?");
    params.push(((page.limit + 1) as i64).into());
    params.push((page.offset as i64).into());

    let rows = conn
        .prepare_cached(&sql)?
        .query_map(params_from_iter(params), |row| {
            let text: Option<String> = row.get(0)?;
            hit(row, snippet::preview(text.as_deref()))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let (rows, has_more) = page.cut(rows);
    Ok(SearchPage {
        hits: rows.into_iter().map(finish).collect::<Result<_, _>>()?,
        has_more,
    })
}

/// Only short words: `LIKE` over the most recently used clips, newest first.
fn search_recent(
    conn: &Connection,
    query: &SearchQuery,
    terms: &Terms,
    page: Page,
) -> StorageResult<SearchPage> {
    let mut params = Vec::new();
    let mut sql = format!(
        "SELECT c.plain_text, {HIT_COLUMNS} FROM
           (SELECT c.id FROM clips c WHERE 1"
    );
    filters(query, true, &mut sql, &mut params);
    sql.push_str(
        " ORDER BY c.last_used_at DESC, c.id DESC LIMIT ?) recent
         JOIN clips c ON c.id = recent.id WHERE 1",
    );
    params.push((SHORT_SCAN as i64).into());
    like_filters(terms, &mut sql, &mut params);
    sql.push_str(" ORDER BY c.last_used_at DESC, c.id DESC LIMIT ? OFFSET ?");
    params.push(((page.limit + 1) as i64).into());
    params.push((page.offset as i64).into());

    let rows = conn
        .prepare_cached(&sql)?
        .query_map(params_from_iter(params), |row| {
            let text: Option<String> = row.get(0)?;
            hit(row, snippet::around(text.as_deref(), &terms.short))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let (rows, has_more) = page.cut(rows);
    Ok(SearchPage {
        hits: rows.into_iter().map(finish).collect::<Result<_, _>>()?,
        has_more,
    })
}

/// Long words: rank the newest index matches, then make snippets for one page of them.
fn search_index(
    conn: &Connection,
    query: &SearchQuery,
    terms: &Terms,
    page: Page,
) -> StorageResult<SearchPage> {
    let fts = terms.fts_query();

    // FTS5 reads matches newest first straight off the index, and stops at the cap. The CROSS
    // JOIN keeps the index as the outer loop even when a filter has an index of its own.
    // bm25() would count every match in the index first, which for a common fragment costs
    // more than the whole typing budget, so the lengths are read for rank::rank instead.
    let mut params: Vec<Value> = vec![fts.clone().into()];
    let mut sql = String::from(
        "SELECT clips_fts.rowid, c.last_used_at,
                coalesce(octet_length(c.plain_text), 0) + coalesce(octet_length(c.source_app_name), 0)
         FROM clips_fts CROSS JOIN clips c ON c.id = clips_fts.rowid
         WHERE clips_fts MATCH ?",
    );
    filters(query, true, &mut sql, &mut params);
    like_filters(terms, &mut sql, &mut params);
    sql.push_str(" ORDER BY clips_fts.rowid DESC LIMIT ?");
    params.push((MAX_CANDIDATES as i64).into());
    let candidates = conn
        .prepare_cached(&sql)?
        .query_map(params_from_iter(params), |row| {
            Ok(Candidate {
                id: row.get(0)?,
                last_used_at: row.get(1)?,
                length: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Fewer than the cap means the scan went through every match: they're rare, or the
    // filters passed few of them.
    let sparse = candidates.len() < MAX_CANDIDATES;
    let ranked = rank::rank(candidates);
    let ids: Vec<i64> = ranked
        .into_iter()
        .skip(page.offset)
        .take(page.limit + 1)
        .collect();
    let (ids, has_more) = page.cut(ids);
    let (Some(&low), Some(&high)) = (ids.iter().min(), ids.iter().max()) else {
        return Ok(SearchPage {
            hits: Vec::new(),
            has_more,
        });
    };

    // Each lookup by rowid seeks every trigram again, which costs about as much as reading a
    // few hundred matches in order. So dense matches get their snippets from one pass over the
    // range the page spans, where the `+` keeps the id list from being used as a lookup, and
    // sparse ones are looked up one by one.
    let select = format!(
        "SELECT snippet(clips_fts, 0, '{open}', '{close}', '…', {tokens}), {HIT_COLUMNS}
         FROM clips_fts CROSS JOIN clips c ON c.id = clips_fts.rowid
         WHERE clips_fts MATCH ?1",
        open = snippet::OPEN,
        close = snippet::CLOSE,
        tokens = snippet::TOKENS,
    );
    let read = |row: &Row<'_>| {
        let marked: Option<String> = row.get(0)?;
        hit(row, snippet::parse(marked.as_deref().unwrap_or_default()))
    };
    let mut rows = Vec::with_capacity(ids.len());
    if sparse {
        let mut stmt = conn.prepare_cached(&format!("{select} AND clips_fts.rowid = ?2"))?;
        for &id in &ids {
            rows.push(stmt.query_row(rusqlite::params![fts, id], read)?);
        }
    } else {
        let mut stmt = conn.prepare_cached(&format!(
            "{select} AND clips_fts.rowid BETWEEN ?2 AND ?3
               AND +clips_fts.rowid IN (SELECT value FROM json_each(?4))"
        ))?;
        let list = serde_json::Value::from(ids.clone()).to_string();
        rows = stmt
            .query_map(rusqlite::params![fts, low, high, list], read)?
            .collect::<Result<_, _>>()?;
        rows.sort_by_key(|(hit, ..)| ids.iter().position(|&id| id == hit.id));
    }
    Ok(SearchPage {
        hits: rows.into_iter().map(finish).collect::<Result<_, _>>()?,
        has_more,
    })
}
