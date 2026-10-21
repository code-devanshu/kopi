//! Snippets: text the user writes once and pastes from the panel. See
//! `docs/SNIPPETS_DESIGN.md`.
//!
//! The store keeps the body as typed and checks lengths. It never reads the placeholders: the
//! panel fills them in when it pastes. Retention and Clear history never touch snippets.

use rusqlite::{OptionalExtension, TransactionBehavior, params};

use super::pinboards::check_label;
use super::{
    Snippet, StorageError, StorageResult, Store, Tombstone, next_seq, now_ms, put_tombstone,
};

/// The longest snippet name, in characters.
pub const MAX_SNIPPET_NAME_CHARS: usize = 64;

/// The longest keyword, in characters.
pub const MAX_SNIPPET_KEYWORD_CHARS: usize = 32;

/// The longest body, in bytes of UTF-8.
pub const MAX_SNIPPET_BODY_BYTES: usize = 64 * 1024;

/// What the user typed into a snippet's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnippetEdit<'a> {
    pub name: &'a str,
    /// None, or only spaces, for no keyword.
    pub keyword: Option<&'a str>,
    pub body: &'a str,
}

/// An edit that passed the checks.
struct Checked {
    name: String,
    keyword: Option<String>,
    body: String,
}

/// The columns [`snippet`] reads.
const SNIPPET_COLUMNS: &str = "id, uuid, name, keyword, body, position, created_at, updated_at,
     last_used_at, use_count";

/// The order of the Snippets tab.
const ORDER: &str = "ORDER BY position, updated_hlc DESC, id";

fn snippet(row: &rusqlite::Row<'_>) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: row.get(0)?,
        uuid: row.get(1)?,
        name: row.get(2)?,
        keyword: row.get(3)?,
        body: row.get(4)?,
        position: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        last_used_at: row.get(8)?,
        use_count: row.get(9)?,
    })
}

impl Store {
    /// Every snippet, in the user's order.
    pub fn snippets(&self) -> StorageResult<Vec<Snippet>> {
        self.with_reader(|conn| {
            let snippets = conn
                .prepare_cached(&format!("SELECT {SNIPPET_COLUMNS} FROM snippets {ORDER}"))?
                .query_map([], snippet)?
                .collect::<Result<_, _>>()?;
            Ok(snippets)
        })
    }

    /// One snippet, if it exists.
    pub fn snippet(&self, id: i64) -> StorageResult<Option<Snippet>> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(&format!(
                    "SELECT {SNIPPET_COLUMNS} FROM snippets WHERE id = ?1"
                ))?
                .query_row([id], snippet)
                .optional()?)
        })
    }

    /// The snippets for the panel's search. Those whose keyword is exactly `text` come first,
    /// then those with every word of `text` in the name, keyword or body, case aside, each in
    /// the user's order. No text lists them all.
    pub fn find_snippets(&self, text: &str) -> StorageResult<Vec<Snippet>> {
        let all = self.snippets()?;
        let words: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
        let (mut first, rest): (Vec<Snippet>, Vec<Snippet>) =
            all.into_iter().partition(|s| keyword_is(s, text));
        first.extend(rest.into_iter().filter(|s| has_words(s, &words)));
        Ok(first)
    }

    /// The snippets whose keyword is exactly `text`, case and spaces around it aside. The
    /// History tab shows them above the clips.
    pub fn snippets_with_keyword(&self, text: &str) -> StorageResult<Vec<Snippet>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        let all = self.snippets()?;
        Ok(all.into_iter().filter(|s| keyword_is(s, text)).collect())
    }

    /// Adds a snippet at the top of the list.
    pub fn create_snippet(&self, edit: SnippetEdit<'_>) -> StorageResult<Snippet> {
        let edit = check(edit)?;
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let id: i64 = tx
            .prepare_cached(
                "INSERT INTO snippets (uuid, name, keyword, body, position, created_at, updated_at,
                                       updated_hlc, sync_seq)
                 VALUES (?1, ?2, ?3, ?4, (SELECT coalesce(min(position) - 1, 0) FROM snippets),
                         ?5, ?5, ?6, ?7)
                 RETURNING id",
            )?
            .query_row(
                params![
                    uuid::Uuid::now_v7().to_string(),
                    edit.name,
                    edit.keyword,
                    edit.body,
                    now,
                    self.clock.tick(now),
                    next_seq(&tx)?
                ],
                |row| row.get(0),
            )?;
        tx.commit()?;
        drop(conn);
        self.changed();
        self.snippet(id)?
            .ok_or_else(|| StorageError::Corrupt(format!("snippet {id} vanished")))
    }

    /// Sets a snippet's name, keyword and body. An edit that changes nothing writes nothing,
    /// since the editor saves as the user types. Returns `false` if there's no such snippet.
    pub fn update_snippet(&self, id: i64, edit: SnippetEdit<'_>) -> StorageResult<bool> {
        let edit = check(edit)?;
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = tx
            .prepare_cached("SELECT EXISTS (SELECT 1 FROM snippets WHERE id = ?1)")?
            .query_row([id], |row| row.get(0))?;
        if !exists {
            return Ok(false);
        }
        let changed = tx
            .prepare_cached(
                "UPDATE snippets SET name = ?2, keyword = ?3, body = ?4, updated_at = ?5,
                                     updated_hlc = ?6, sync_seq = ?7
                 WHERE id = ?1 AND (name IS NOT ?2 OR keyword IS NOT ?3 OR body IS NOT ?4)",
            )?
            .execute(params![
                id,
                edit.name,
                edit.keyword,
                edit.body,
                now,
                self.clock.tick(now),
                next_seq(&tx)?
            ])?;
        if changed == 0 {
            // Unchanged. The sync_seq taken for it is rolled back with the transaction.
            return Ok(true);
        }
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(true)
    }

    /// Puts the snippets in `ids` first, in that order. Any left out keep their order after
    /// them, and unknown ids are skipped. Each snippet that moves is a change to sync.
    pub fn reorder_snippets(&self, ids: &[i64]) -> StorageResult<()> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Vec<(i64, i64)> = tx
            .prepare_cached(&format!("SELECT id, position FROM snippets {ORDER}"))?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let known = |id: &i64| current.iter().any(|(c, _)| c == id);
        let mut order: Vec<i64> = Vec::with_capacity(current.len());
        for id in ids.iter().chain(current.iter().map(|(id, _)| id)) {
            if known(id) && !order.contains(id) {
                order.push(*id);
            }
        }
        let mut moved = 0;
        {
            let now = now_ms();
            let mut update = tx.prepare_cached(
                "UPDATE snippets SET position = ?2, updated_at = ?3, updated_hlc = ?4, sync_seq = ?5
                 WHERE id = ?1",
            )?;
            for (position, id) in order.iter().enumerate() {
                let old = current.iter().find(|(c, _)| c == id).map(|(_, p)| *p);
                if old == Some(position as i64) {
                    continue;
                }
                moved += update.execute(params![
                    id,
                    position as i64,
                    now,
                    self.clock.tick(now),
                    next_seq(&tx)?
                ])?;
            }
        }
        tx.commit()?;
        drop(conn);
        if moved > 0 {
            self.changed();
        }
        Ok(())
    }

    /// Deletes a snippet, leaving a tombstone so the delete reaches the other computers.
    /// Returns `false` if there's no such snippet.
    pub fn delete_snippet(&self, id: i64) -> StorageResult<bool> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let uuid: Option<String> = tx
            .prepare_cached("DELETE FROM snippets WHERE id = ?1 RETURNING uuid")?
            .query_row([id], |row| row.get(0))
            .optional()?;
        let Some(uuid) = uuid else {
            return Ok(false);
        };
        put_tombstone(&tx, Tombstone::SNIPPET, &uuid, self.clock.tick(now_ms()))?;
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(true)
    }

    /// Counts a paste or copy of a snippet. Use is kept on this computer only, so it isn't a
    /// change to sync. Returns `false` if there's no such snippet.
    pub fn mark_snippet_used(&self, id: i64, now: i64) -> StorageResult<bool> {
        let changed = self
            .writer()
            .prepare_cached(
                "UPDATE snippets SET last_used_at = ?2, use_count = use_count + 1 WHERE id = ?1",
            )?
            .execute(params![id, now])?;
        Ok(changed > 0)
    }

    /// The id of the newest clip with text to paste: text, rich text, a link, code or a color.
    /// A snippet's `{clipboard}` pastes it.
    pub fn newest_text_clip(&self) -> StorageResult<Option<i64>> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(
                    "SELECT id FROM clips
                     WHERE plain_text IS NOT NULL AND plain_text != ''
                       AND kind IN ('text', 'rich_text', 'link', 'code', 'color')
                     ORDER BY last_used_at DESC, id DESC LIMIT 1",
                )?
                .query_row([], |row| row.get(0))
                .optional()?)
        })
    }
}

/// Whether `text` is exactly the snippet's keyword, case and spaces around it aside.
fn keyword_is(snippet: &Snippet, text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && snippet
            .keyword
            .as_deref()
            .is_some_and(|k| k.to_lowercase() == text.to_lowercase())
}

/// Whether every one of `words`, already lowercase, is in the snippet's name, keyword or body.
fn has_words(snippet: &Snippet, words: &[String]) -> bool {
    if words.is_empty() {
        return true;
    }
    let haystack = format!(
        "{}\n{}\n{}",
        snippet.name,
        snippet.keyword.as_deref().unwrap_or(""),
        snippet.body
    )
    .to_lowercase();
    words.iter().all(|w| haystack.contains(w.as_str()))
}

fn check(edit: SnippetEdit<'_>) -> StorageResult<Checked> {
    Ok(Checked {
        name: check_label(edit.name, "snippet", MAX_SNIPPET_NAME_CHARS)?,
        keyword: check_keyword(edit.keyword)?,
        body: check_body(edit.body)?,
    })
}

/// A trimmed keyword of 1 to [`MAX_SNIPPET_KEYWORD_CHARS`] characters without spaces, or none.
pub(crate) fn check_keyword(keyword: Option<&str>) -> StorageResult<Option<String>> {
    let Some(keyword) = keyword.map(str::trim).filter(|k| !k.is_empty()) else {
        return Ok(None);
    };
    if keyword.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(StorageError::Invalid("a keyword can't have spaces".into()));
    }
    if keyword.chars().count() > MAX_SNIPPET_KEYWORD_CHARS {
        return Err(StorageError::Invalid(format!(
            "a keyword can be at most {MAX_SNIPPET_KEYWORD_CHARS} characters"
        )));
    }
    Ok(Some(keyword.to_owned()))
}

/// The body as typed, if it fits in [`MAX_SNIPPET_BODY_BYTES`].
pub(crate) fn check_body(body: &str) -> StorageResult<String> {
    if body.len() > MAX_SNIPPET_BODY_BYTES {
        return Err(StorageError::Invalid(format!(
            "a snippet can be at most {} KB",
            MAX_SNIPPET_BODY_BYTES / 1024
        )));
    }
    Ok(body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_are_trimmed_and_have_no_spaces() {
        assert_eq!(check_keyword(None).unwrap(), None);
        assert_eq!(check_keyword(Some("")).unwrap(), None);
        assert_eq!(check_keyword(Some("   ")).unwrap(), None);
        assert_eq!(
            check_keyword(Some(" ;sig ")).unwrap().as_deref(),
            Some(";sig")
        );
        assert_eq!(
            check_keyword(Some("住所")).unwrap().as_deref(),
            Some("住所")
        );
        assert!(check_keyword(Some("a b")).is_err());
        assert!(check_keyword(Some("a\tb")).is_err());
        let longest = "é".repeat(MAX_SNIPPET_KEYWORD_CHARS);
        assert_eq!(
            check_keyword(Some(&longest)).unwrap(),
            Some(longest.clone())
        );
        assert!(check_keyword(Some(&format!("{longest}x"))).is_err());
    }

    #[test]
    fn bodies_are_kept_as_typed_up_to_64_kib() {
        assert_eq!(check_body("").unwrap(), "");
        assert_eq!(
            check_body("  Hi {input:Name},\n").unwrap(),
            "  Hi {input:Name},\n"
        );
        let longest = "a".repeat(MAX_SNIPPET_BODY_BYTES);
        assert_eq!(check_body(&longest).unwrap().len(), MAX_SNIPPET_BODY_BYTES);
        assert!(check_body(&format!("{longest}a")).is_err());
    }
}
