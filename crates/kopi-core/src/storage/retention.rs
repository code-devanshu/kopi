//! Retention limits, clearing history and the orphan sweep. See `docs/DATA_MODEL.md`.
//!
//! Limits only ever remove clips that aren't [kept](super::KEPT), oldest `last_used_at` first.
//! Clearing history can take kept clips too, when the user asks.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior, params};
use serde_json::Value;

use super::{
    KEPT, Removed, StorageError, StorageResult, Store, blobs, delete_in, next_seq, now_ms, thumbs,
};

/// The default for [`Limits::max_total_bytes`]: 2 GiB.
pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

const MAX_ITEMS: &str = "history.max_items";
const MAX_AGE_DAYS: &str = "history.max_age_days";
const MAX_TOTAL_BYTES: &str = "history.max_total_bytes";

pub(crate) const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// How long deletes are remembered for sync.
pub const TOMBSTONE_DAYS: u32 = 90;

/// How much history to keep. `None` is unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most clips to keep, not counting kept ones.
    pub max_items: Option<u64>,
    /// Clips last used longer ago than this are removed.
    pub max_age_days: Option<u32>,
    /// The most bytes all clips may take: their text and every format, in the database or in
    /// a blob file. Kept clips count towards it, but only other clips are removed.
    pub max_total_bytes: Option<u64>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_items: None,
            max_age_days: None,
            max_total_bytes: Some(DEFAULT_MAX_TOTAL_BYTES),
        }
    }
}

impl Limits {
    /// Whether any limit is lower than in `old`, so retention should run now.
    pub fn is_stricter_than(&self, old: &Limits) -> bool {
        fn lower<T: Ord>(new: Option<T>, old: Option<T>) -> bool {
            match (new, old) {
                (Some(new), Some(old)) => new < old,
                (Some(_), None) => true,
                (None, _) => false,
            }
        }
        lower(self.max_items, old.max_items)
            || lower(self.max_age_days, old.max_age_days)
            || lower(self.max_total_bytes, old.max_total_bytes)
    }

    fn check(&self) -> StorageResult<()> {
        let zero = self.max_items == Some(0)
            || self.max_age_days == Some(0)
            || self.max_total_bytes == Some(0);
        if zero {
            return Err(StorageError::Invalid(
                "History limits must be at least 1.".into(),
            ));
        }
        Ok(())
    }
}

impl Store {
    /// The limits in the settings. A missing or invalid value gets its default.
    pub fn retention_limits(&self) -> StorageResult<Limits> {
        let defaults = Limits::default();
        Ok(Limits {
            max_items: limit(self.setting(MAX_ITEMS)?, defaults.max_items),
            max_age_days: limit(self.setting(MAX_AGE_DAYS)?, defaults.max_age_days),
            max_total_bytes: limit(self.setting(MAX_TOTAL_BYTES)?, defaults.max_total_bytes),
        })
    }

    /// Saves the limits. Every limit must be at least 1. It doesn't apply them: run
    /// [`apply_retention`](Self::apply_retention) after, when they're
    /// [stricter](Limits::is_stricter_than).
    pub fn set_retention_limits(&self, limits: &Limits) -> StorageResult<()> {
        limits.check()?;
        let json = |value: Option<u64>| value.map_or(Value::Null, Value::from);
        self.set_settings(&[
            (MAX_ITEMS, json(limits.max_items)),
            (MAX_AGE_DAYS, json(limits.max_age_days.map(u64::from))),
            (MAX_TOTAL_BYTES, json(limits.max_total_bytes)),
        ])
    }

    /// Removes the clips over `limits` at time `now` (Unix milliseconds), in one transaction:
    /// those too old, then the oldest beyond the count, then the oldest until the total size
    /// fits. Kept clips are never removed. Returns the ids removed.
    pub fn apply_retention(&self, limits: &Limits, now: i64) -> StorageResult<Vec<i64>> {
        if *limits
            == (Limits {
                max_items: None,
                max_age_days: None,
                max_total_bytes: None,
            })
        {
            return Ok(Vec::new());
        }
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut removed = Removed::default();

        if let Some(days) = limits.max_age_days {
            let cutoff = now.saturating_sub(i64::from(days) * DAY_MS);
            let ids = ids(
                &tx,
                &format!("SELECT c.id FROM clips c WHERE c.last_used_at < ?1 AND NOT {KEPT}"),
                params![cutoff],
            )?;
            removed.extend(delete_in(&tx, ids)?);
        }

        if let Some(max) = limits.max_items {
            let ids = ids(
                &tx,
                &format!(
                    "SELECT c.id FROM clips c WHERE NOT {KEPT}
                     ORDER BY c.last_used_at DESC, c.id DESC LIMIT -1 OFFSET ?1"
                ),
                params![max.min(i64::MAX as u64) as i64],
            )?;
            removed.extend(delete_in(&tx, ids)?);
        }

        if let Some(max) = limits.max_total_bytes {
            let max = max.min(i64::MAX as u64) as i64;
            let total: i64 = tx.query_row(
                "SELECT coalesce(sum(size_bytes), 0) FROM clips",
                [],
                |row| row.get(0),
            )?;
            if total > max {
                let kept: i64 = tx.query_row(
                    &format!("SELECT coalesce(sum(c.size_bytes), 0) FROM clips c WHERE {KEPT}"),
                    [],
                    |row| row.get(0),
                )?;
                // Newest first, each clip with the bytes of it and every newer one. The first
                // that doesn't fit, and everything older, goes.
                let ids = ids(
                    &tx,
                    &format!(
                        "SELECT id FROM (
                           SELECT c.id, sum(c.size_bytes) OVER (
                             ORDER BY c.last_used_at DESC, c.id DESC) AS running
                           FROM clips c WHERE NOT {KEPT})
                         WHERE running + ?1 > ?2"
                    ),
                    params![kept, max],
                )?;
                removed.extend(delete_in(&tx, ids)?);
            }
        }

        tx.commit()?;
        self.remove_files(conn, removed)
    }

    /// Deletes the clips last used at or after `since` (Unix milliseconds), or every clip when
    /// it's none. With `keep_kept`, pinned clips and those on a pinboard stay. Then compacts
    /// the database, so the deleted content doesn't linger on disk. Returns the ids removed.
    ///
    /// It leaves a range tombstone, so the other computers clear the same clips.
    pub fn clear_history(&self, since: Option<i64>, keep_kept: bool) -> StorageResult<Vec<i64>> {
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let hlc = self.clock.tick(now);
        tx.prepare_cached(
            "INSERT INTO clear_ranges (from_ms, to_ms, include_kept, hlc, sync_seq)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?
        .execute(params![since, i64::MAX, !keep_kept, hlc, next_seq(&tx)?])?;
        // As a range from another computer is applied: removed clips get tombstones, and kept
        // ones a new HLC, so every computer ends up with the same. Its window has no end: the
        // HLC already leaves out what's used after it, and a clock ahead of this one doesn't.
        let mut removed = Removed::default();
        self.settle_range(&tx, hlc, since, i64::MAX, &mut removed)?;
        tx.commit()?;
        let ids = self.remove_files(conn, removed)?;
        self.changed();
        if !ids.is_empty() {
            self.compact()?;
        }
        Ok(ids)
    }

    /// Drops tombstones and cleared ranges older than [`TOMBSTONE_DAYS`]. A computer that
    /// hasn't synced for that long could miss those deletes.
    pub fn prune_tombstones(&self, now: i64) -> StorageResult<()> {
        let cutoff = super::hlc::from_ms(now - i64::from(TOMBSTONE_DAYS) * DAY_MS);
        let conn = self.writer();
        conn.prepare_cached("DELETE FROM tombstones WHERE hlc < ?1")?
            .execute([cutoff])?;
        conn.prepare_cached("DELETE FROM clear_ranges WHERE hlc < ?1")?
            .execute([cutoff])?;
        Ok(())
    }

    /// Rebuilds the database without its free pages, then empties the WAL file. `secure_delete`
    /// already zeroes what's deleted; this also drops the old copies of pages in the WAL.
    pub fn compact(&self) -> StorageResult<()> {
        let conn = self.writer();
        conn.execute_batch("VACUUM")?;
        // Returns (busy, wal frames, frames checkpointed). Busy means a reader held it up;
        // the next checkpoint finishes the job.
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }

    /// Deletes the files in `blobs/` and `thumbs/` that no clip uses, and the temporary files
    /// an earlier run left behind. A crash between writing a file and committing its row
    /// leaves these. Returns how many files were deleted.
    pub fn sweep_orphans(&self) -> StorageResult<usize> {
        // Held throughout, so no clip starts using a blob while it's being checked.
        let conn = self.writer();
        let mut count = 0;
        let mut blob_used =
            conn.prepare_cached("SELECT EXISTS (SELECT 1 FROM clip_formats WHERE blob_hash = ?1)")?;
        for path in files(self.blobs.root())? {
            let orphan = match file_name(&path) {
                Some(name) if is_stale_temp(name) => true,
                Some(name) => match name.strip_suffix(".bin") {
                    Some(hash) if blobs::check_hash(hash).is_ok() => {
                        !blob_used.query_row([hash], |row| row.get::<_, bool>(0))?
                    }
                    _ => false,
                },
                None => false,
            };
            if orphan && remove(&path)? {
                count += 1;
            }
        }

        let mut clip_exists =
            conn.prepare_cached("SELECT EXISTS (SELECT 1 FROM clips WHERE content_hash = ?1)")?;
        let current = format!("{}.png", thumbs::THUMBNAIL_SIZE);
        for path in files(&self.thumbs_dir)? {
            let orphan = match file_name(&path) {
                Some(name) if is_stale_temp(name) => true,
                // Thumbnails of another size are from an older Kopi.
                Some(name) => match name.split_once('@') {
                    Some((hash, rest)) if blobs::check_hash(hash).is_ok() => {
                        rest != current
                            || !clip_exists.query_row([hash], |row| row.get::<_, bool>(0))?
                    }
                    _ => false,
                },
                None => false,
            };
            if orphan && remove(&path)? {
                count += 1;
            }
        }
        Ok(count)
    }
}

/// Reads a limit from its setting: a positive number, or null for unlimited.
fn limit<T: TryFrom<u64>>(value: Option<Value>, default: Option<T>) -> Option<T> {
    match value {
        None => default,
        Some(Value::Null) => None,
        Some(value) => match value.as_u64().filter(|&n| n > 0).map(T::try_from) {
            Some(Ok(n)) => Some(n),
            _ => default,
        },
    }
}

fn ids(conn: &Connection, sql: &str, params: impl rusqlite::Params) -> StorageResult<Vec<i64>> {
    Ok(conn
        .prepare(sql)?
        .query_map(params, |row| row.get(0))?
        .collect::<Result<_, _>>()?)
}

/// Every file under `dir`, at any depth.
fn files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let entries = match fs::read_dir(&dir) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            entries => entries?,
        };
        for entry in entries {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                dirs.push(entry.path());
            } else if kind.is_file() {
                found.push(entry.path());
            }
        }
    }
    Ok(found)
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()
}

/// A temporary file from [`blobs::write_atomic`] that another process left behind. This
/// process's own may still be being written.
fn is_stale_temp(name: &str) -> bool {
    let Some((_, rest)) = name.split_once(".tmp-") else {
        return false;
    };
    let pid = rest
        .split('-')
        .next()
        .and_then(|pid| pid.parse::<u32>().ok());
    pid != Some(std::process::id())
}

/// Deletes a file. Returns `false` if it was already gone.
fn remove(path: &Path) -> io::Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stricter_limits() {
        let none = Limits {
            max_items: None,
            max_age_days: None,
            max_total_bytes: None,
        };
        let some = Limits {
            max_items: Some(100),
            ..none
        };
        assert!(some.is_stricter_than(&none));
        assert!(!none.is_stricter_than(&some));
        assert!(!some.is_stricter_than(&some));
        let fewer = Limits {
            max_items: Some(50),
            ..none
        };
        assert!(fewer.is_stricter_than(&some));
        let looser_count_but_age = Limits {
            max_items: Some(200),
            max_age_days: Some(7),
            ..none
        };
        assert!(looser_count_but_age.is_stricter_than(&some));
    }

    #[test]
    fn reads_limits_leniently() {
        assert_eq!(limit::<u64>(None, Some(5)), Some(5));
        assert_eq!(limit::<u64>(Some(Value::Null), Some(5)), None);
        assert_eq!(limit::<u64>(Some(Value::from(9)), None), Some(9));
        assert_eq!(limit::<u64>(Some(Value::from(0)), Some(5)), Some(5));
        assert_eq!(limit::<u64>(Some(Value::from("x")), None), None);
        assert_eq!(limit::<u32>(Some(Value::from(u64::MAX)), Some(5)), Some(5));
    }

    #[test]
    fn stale_temp_files() {
        assert!(is_stale_temp("abc.tmp-1-0"));
        assert!(!is_stale_temp(&format!("abc.tmp-{}-3", std::process::id())));
        assert!(!is_stale_temp("abc.bin"));
    }
}
