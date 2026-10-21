//! Pins and pinboards. See `docs/DATA_MODEL.md`.
//!
//! `pinned` is the quick "keep this" toggle on a clip. Pinboards are named, ordered collections
//! the user keeps. A clip can be on several. Retention never removes a pinned clip or one on a
//! pinboard; an explicit delete still does.

use rusqlite::{OptionalExtension, TransactionBehavior, params};

use super::{
    Pinboard, StorageError, StorageResult, Store, Tombstone, next_seq, now_ms, put_tombstone,
};

/// The longest pinboard name, in characters.
pub const MAX_PINBOARD_NAME_CHARS: usize = 64;

/// The columns [`pinboard`] reads.
const PINBOARD_COLUMNS: &str =
    "b.id, b.uuid, b.name, b.color, b.position, b.created_at, b.updated_at,
     (SELECT count(*) FROM pinboard_items i WHERE i.pinboard_id = b.id)";

fn pinboard(row: &rusqlite::Row<'_>) -> rusqlite::Result<Pinboard> {
    Ok(Pinboard {
        id: row.get(0)?,
        uuid: row.get(1)?,
        name: row.get(2)?,
        color: row.get(3)?,
        position: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        clip_count: row.get(7)?,
    })
}

impl Store {
    /// Pins or unpins a clip. Returns `false` if there's no such clip.
    pub fn set_pinned(&self, clip_id: i64, pinned: bool) -> StorageResult<bool> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx
            .prepare_cached(
                "UPDATE clips SET pinned = ?2, pinned_hlc = ?3, sync_seq = ?4 WHERE id = ?1",
            )?
            .execute(params![
                clip_id,
                pinned,
                self.clock.tick(now_ms()),
                next_seq(&tx)?
            ])?;
        if changed == 0 {
            return Ok(false);
        }
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(true)
    }

    /// Every pinboard, in the user's order, with how many clips each holds.
    pub fn pinboards(&self) -> StorageResult<Vec<Pinboard>> {
        self.with_reader(|conn| {
            let boards = conn
                .prepare_cached(&format!(
                    "SELECT {PINBOARD_COLUMNS} FROM pinboards b
                     ORDER BY b.position, b.updated_hlc DESC, b.id"
                ))?
                .query_map([], pinboard)?
                .collect::<Result<_, _>>()?;
            Ok(boards)
        })
    }

    /// One pinboard, if it exists.
    pub fn pinboard(&self, id: i64) -> StorageResult<Option<Pinboard>> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(&format!(
                    "SELECT {PINBOARD_COLUMNS} FROM pinboards b WHERE b.id = ?1"
                ))?
                .query_row([id], pinboard)
                .optional()?)
        })
    }

    /// Adds a pinboard after the others. The name is trimmed and must not be empty. `color` is
    /// `#RRGGBB` or none.
    pub fn create_pinboard(&self, name: &str, color: Option<&str>) -> StorageResult<Pinboard> {
        let name = check_name(name)?;
        let color = check_color(color)?;
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let id: i64 = tx
            .prepare_cached(
                "INSERT INTO pinboards (uuid, name, color, position, created_at, updated_at,
                                        updated_hlc, sync_seq)
                 VALUES (?1, ?2, ?3, (SELECT coalesce(max(position) + 1, 0) FROM pinboards), ?4, ?4,
                         ?5, ?6)
                 RETURNING id",
            )?
            .query_row(
                params![
                    uuid::Uuid::now_v7().to_string(),
                    name,
                    color,
                    now,
                    self.clock.tick(now),
                    next_seq(&tx)?
                ],
                |row| row.get(0),
            )?;
        tx.commit()?;
        drop(conn);
        self.changed();
        self.pinboard(id)?
            .ok_or_else(|| StorageError::Corrupt(format!("pinboard {id} vanished")))
    }

    /// Renames a pinboard. Returns `false` if there's no such pinboard.
    pub fn rename_pinboard(&self, id: i64, name: &str) -> StorageResult<bool> {
        let name = check_name(name)?;
        self.update_pinboard(id, "name", &name)
    }

    /// Sets a pinboard's color, `#RRGGBB`, or clears it. Returns `false` if there's no such
    /// pinboard.
    pub fn recolor_pinboard(&self, id: i64, color: Option<&str>) -> StorageResult<bool> {
        let color = check_color(color)?;
        self.update_pinboard(id, "color", &color)
    }

    /// Sets one column of a pinboard, as a change to sync.
    fn update_pinboard(
        &self,
        id: i64,
        column: &str,
        value: &dyn rusqlite::ToSql,
    ) -> StorageResult<bool> {
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx
            .prepare_cached(&format!(
                "UPDATE pinboards SET {column} = ?2, updated_at = ?3, updated_hlc = ?4,
                                      sync_seq = ?5
                 WHERE id = ?1"
            ))?
            .execute(params![
                id,
                value,
                now,
                self.clock.tick(now),
                next_seq(&tx)?
            ])?;
        if changed == 0 {
            return Ok(false);
        }
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(true)
    }

    /// Puts the pinboards in `ids` first, in that order. Any left out keep their order after
    /// them, and unknown ids are skipped.
    pub fn reorder_pinboards(&self, ids: &[i64]) -> StorageResult<()> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Vec<i64> = tx
            .prepare_cached("SELECT id FROM pinboards ORDER BY position, id")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        let mut order: Vec<i64> = Vec::with_capacity(current.len());
        for &id in ids.iter().chain(&current) {
            if current.contains(&id) && !order.contains(&id) {
                order.push(id);
            }
        }
        let mut moved = 0;
        {
            let now = now_ms();
            let mut update = tx.prepare_cached(
                "UPDATE pinboards SET position = ?2, updated_at = ?3, updated_hlc = ?4,
                                      sync_seq = ?5
                 WHERE id = ?1 AND position IS NOT ?2",
            )?;
            let mut current_position =
                tx.prepare_cached("SELECT position FROM pinboards WHERE id = ?1")?;
            for (position, id) in order.iter().enumerate() {
                let old: i64 = current_position.query_row([id], |row| row.get(0))?;
                if old == position as i64 {
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

    /// Deletes a pinboard. Its clips stay in the history. Returns `false` if there's no such
    /// pinboard.
    pub fn delete_pinboard(&self, id: i64) -> StorageResult<bool> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let uuid: Option<String> = tx
            .prepare_cached("DELETE FROM pinboards WHERE id = ?1 RETURNING uuid")?
            .query_row([id], |row| row.get(0))
            .optional()?;
        let Some(uuid) = uuid else {
            return Ok(false);
        };
        put_tombstone(&tx, Tombstone::PINBOARD, &uuid, self.clock.tick(now_ms()))?;
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(true)
    }

    /// Puts clips at the top of a pinboard, the first of `clip_ids` topmost. Clips already on it
    /// and unknown clips are skipped. Returns how many were added, or `None` if there's no such
    /// pinboard.
    pub fn add_to_pinboard(&self, id: i64, clip_ids: &[i64]) -> StorageResult<Option<usize>> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = tx
            .prepare_cached("SELECT EXISTS (SELECT 1 FROM pinboards WHERE id = ?1)")?
            .query_row([id], |row| row.get(0))?;
        if !exists {
            return Ok(None);
        }
        let now = now_ms();
        let mut added = 0;
        {
            // The board lists by ascending position, so each new clip goes one above the top.
            // Adding the last one first leaves `clip_ids` in order.
            let mut insert = tx.prepare_cached(
                "INSERT OR IGNORE INTO pinboard_items (pinboard_id, clip_id, position, added_at,
                                                       hlc, sync_seq)
                 SELECT ?1, c.id,
                        (SELECT coalesce(min(position) - 1, 0) FROM pinboard_items WHERE pinboard_id = ?1),
                        ?3, ?4, ?5
                 FROM clips c WHERE c.id = ?2",
            )?;
            for &clip_id in clip_ids.iter().rev() {
                added += insert.execute(params![
                    id,
                    clip_id,
                    now,
                    self.clock.tick(now),
                    next_seq(&tx)?
                ])?;
            }
        }
        if added > 0 {
            touch(&tx, id, now)?;
        }
        tx.commit()?;
        drop(conn);
        if added > 0 {
            self.changed();
        }
        Ok(Some(added))
    }

    /// Takes clips off a pinboard. They stay in the history. Returns how many were removed.
    pub fn remove_from_pinboard(&self, id: i64, clip_ids: &[i64]) -> StorageResult<usize> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut removed = 0;
        {
            let mut delete = tx.prepare_cached(
                "DELETE FROM pinboard_items WHERE pinboard_id = ?1 AND clip_id = ?2
                 RETURNING (SELECT uuid FROM pinboards WHERE id = ?1),
                           (SELECT content_hash FROM clips WHERE id = ?2)",
            )?;
            let hlc = self.clock.tick(now_ms());
            for &clip_id in clip_ids {
                let keys: Option<(String, String)> = delete
                    .query_row(params![id, clip_id], |row| Ok((row.get(0)?, row.get(1)?)))
                    .optional()?;
                if let Some((board, clip)) = keys {
                    let key = Tombstone::item_key(&board, &clip);
                    put_tombstone(&tx, Tombstone::PINBOARD_ITEM, &key, hlc)?;
                    removed += 1;
                }
            }
        }
        if removed > 0 {
            touch(&tx, id, now_ms())?;
        }
        tx.commit()?;
        drop(conn);
        if removed > 0 {
            self.changed();
        }
        Ok(removed)
    }

    /// The pinboards a clip is on, in the user's order.
    pub fn clip_pinboards(&self, clip_id: i64) -> StorageResult<Vec<i64>> {
        self.with_reader(|conn| {
            let ids = conn
                .prepare_cached(
                    "SELECT b.id FROM pinboard_items i JOIN pinboards b ON b.id = i.pinboard_id
                     WHERE i.clip_id = ?1 ORDER BY b.position, b.id",
                )?
                .query_map([clip_id], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(ids)
        })
    }

    /// Whether retention must leave a clip alone: it's pinned or on a pinboard. `false` for a
    /// clip that doesn't exist.
    pub fn is_kept(&self, clip_id: i64) -> StorageResult<bool> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(&format!(
                    "SELECT EXISTS (SELECT 1 FROM clips c WHERE c.id = ?1 AND {KEPT})"
                ))?
                .query_row([clip_id], |row| row.get(0))?)
        })
    }
}

/// An SQL condition on `clips c`: the clip is pinned or on a pinboard, so retention keeps it.
pub const KEPT: &str =
    "(c.pinned = 1 OR EXISTS (SELECT 1 FROM pinboard_items k WHERE k.clip_id = c.id))";

/// Marks a pinboard's contents changed. Items sync on their own, so this isn't a change to
/// the pinboard's row.
fn touch(conn: &rusqlite::Connection, id: i64, now: i64) -> rusqlite::Result<()> {
    conn.prepare_cached("UPDATE pinboards SET updated_at = ?2 WHERE id = ?1")?
        .execute(params![id, now])?;
    Ok(())
}

/// A trimmed name of 1 to [`MAX_PINBOARD_NAME_CHARS`] characters, without control characters.
pub(crate) fn check_name(name: &str) -> StorageResult<String> {
    check_label(name, "pinboard", MAX_PINBOARD_NAME_CHARS)
}

/// `name` trimmed and without control characters, if it has 1 to `max` characters. `what` is
/// the thing it names, for the error.
pub(crate) fn check_label(name: &str, what: &str, max: usize) -> StorageResult<String> {
    let name: String = name.trim().chars().filter(|c| !c.is_control()).collect();
    match name.chars().count() {
        0 => Err(StorageError::Invalid(format!("a {what} needs a name"))),
        n if n > max => Err(StorageError::Invalid(format!(
            "a {what} name can be at most {max} characters"
        ))),
        _ => Ok(name),
    }
}

/// `#RRGGBB` in capitals, or none.
pub(crate) fn check_color(color: Option<&str>) -> StorageResult<Option<String>> {
    let Some(color) = color else {
        return Ok(None);
    };
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(StorageError::Invalid(format!(
            "not a #RRGGBB color: {color:?}"
        )));
    }
    Ok(Some(color.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_limited() {
        assert_eq!(check_name("  Work  ").unwrap(), "Work");
        assert_eq!(check_name("a\tb\nc").unwrap(), "abc");
        assert_eq!(check_name("東京").unwrap(), "東京");
        assert!(check_name("").is_err());
        assert!(check_name(" \n\t ").is_err());
        let longest = "é".repeat(MAX_PINBOARD_NAME_CHARS);
        assert_eq!(check_name(&longest).unwrap(), longest);
        assert!(check_name(&format!("{longest}x")).is_err());
    }

    #[test]
    fn colors_are_hex() {
        assert_eq!(check_color(None).unwrap(), None);
        assert_eq!(
            check_color(Some("#d08a3a")).unwrap().as_deref(),
            Some("#D08A3A")
        );
        for bad in [
            "",
            "#",
            "d08a3a",
            "#d08a3",
            "#d08a3a0",
            "#g08a3a",
            "red",
            "#éé8a3a",
        ] {
            assert!(check_color(Some(bad)).is_err(), "{bad:?}");
        }
    }
}
