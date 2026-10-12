//! The SQLite database, the blob files and the thumbnails. See `docs/DATA_MODEL.md`.
//!
//! ```text
//! <data dir>/
//! ├── kopi.db      SQLite in WAL mode, plus kopi.db-wal and kopi.db-shm
//! ├── blobs/       format data over 64 KiB, named by BLAKE3 hash
//! └── thumbs/      320 px PNG thumbnails of image clips
//! ```
//!
//! One connection writes, behind a mutex. Reads use a small pool of read-only connections, so
//! they never wait for a write.

mod blobs;
mod migrate;
mod model;
mod thumbs;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};

pub use blobs::BlobStore;
pub use migrate::SCHEMA_VERSION;
pub use model::{Clip, ClipFormat, ClipKind, NewClip, NewFormat, Platform, Upserted};
pub use thumbs::{THUMBNAIL_SIZE, ThumbnailDone, ThumbnailListener};

use thumbs::{Source, SourceFormat, Thumbnailer};

/// Format data up to this size is stored in the row. Anything larger goes to a blob file.
pub const MAX_INLINE_BYTES: usize = 64 * 1024;

/// `plain_text` is cut to this many bytes. The full text stays in `clip_formats`.
pub const MAX_TEXT_INDEX_BYTES: usize = 64 * 1024;

/// How many idle read connections to keep open.
const MAX_IDLE_READERS: usize = 4;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("file error: {0}")]
    Io(#[from] io::Error),
    #[error(
        "the database was made by a newer Kopi (schema {found}, this build knows up to {supported})"
    )]
    NewerSchema { found: u32, supported: u32 },
    #[error("not a BLAKE3 hex hash: {0:?}")]
    InvalidHash(String),
    #[error("stored data is invalid: {0}")]
    Corrupt(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

pub struct Store {
    db_path: PathBuf,
    writer: Mutex<Connection>,
    readers: Mutex<Vec<Connection>>,
    blobs: BlobStore,
    thumbs_dir: PathBuf,
    thumbnailer: Thumbnailer,
}

impl Store {
    /// Opens the store in `dir`, creating it if needed, and brings the schema up to date.
    pub fn open(dir: impl AsRef<Path>) -> StorageResult<Self> {
        Self::open_with_listener(dir, Box::new(|_| {}))
    }

    /// Like [`open`](Self::open), and calls `listener` on the thumbnail thread each time a
    /// thumbnail is written or fails.
    pub fn open_with_listener(
        dir: impl AsRef<Path>,
        listener: ThumbnailListener,
    ) -> StorageResult<Self> {
        let dir = dir.as_ref();
        create_private_dir(dir)?;
        let blobs_dir = dir.join("blobs");
        let thumbs_dir = dir.join("thumbs");
        fs::create_dir_all(&blobs_dir)?;
        fs::create_dir_all(&thumbs_dir)?;

        let db_path = dir.join("kopi.db");
        let mut writer = Connection::open(&db_path)?;
        configure_writer(&writer)?;
        migrate::migrate(&mut writer)?;

        Ok(Self {
            db_path,
            writer: Mutex::new(writer),
            readers: Mutex::new(Vec::new()),
            blobs: BlobStore::new(blobs_dir),
            thumbnailer: Thumbnailer::spawn(thumbs_dir.clone(), listener)?,
            thumbs_dir,
        })
    }

    pub fn blobs(&self) -> &BlobStore {
        &self.blobs
    }

    /// Adds a clip, or moves the existing clip with the same content hash to the top.
    pub fn upsert_clip(&self, clip: NewClip) -> StorageResult<Upserted> {
        self.upsert_clip_at(clip, now_ms())
    }

    /// [`upsert_clip`](Self::upsert_clip) with an explicit time, in Unix milliseconds.
    ///
    /// - A new clip gets its formats stored, with data over 64 KiB written to blob files before
    ///   the transaction. Image clips get a thumbnail queued.
    /// - An existing clip gets `last_used_at`, `use_count` and the source app updated. Rich text
    ///   also gets its formats replaced, since the formatting may have changed.
    /// - A plain-text format that is an exact copy of `plain_text` isn't stored. Pass the full
    ///   text as a format when `plain_text` would be cut, or the rest is lost.
    pub fn upsert_clip_at(&self, clip: NewClip, now: i64) -> StorageResult<Upserted> {
        blobs::check_hash(&clip.content_hash)?;
        let (plain_text, truncated) = match clip.plain_text {
            Some(text) => {
                let (text, truncated) = truncate_text(text);
                (Some(text), truncated)
            }
            None => (None, false),
        };
        let plain_text_format = clip.platform.plain_text_format();
        let formats: Vec<&NewFormat> = clip
            .formats
            .iter()
            .filter(|f| {
                let duplicate = !truncated
                    && f.format == plain_text_format
                    && plain_text.as_deref().map(str::as_bytes) == Some(&f.data[..]);
                !duplicate
            })
            .collect();
        let size_bytes = plain_text.as_ref().map_or(0, String::len)
            + formats.iter().map(|f| f.data.len()).sum::<usize>();
        let metadata = clip.metadata.as_ref().map(serde_json::Value::to_string);
        let new_uuid = uuid::Uuid::now_v7().to_string();

        let mut conn = self.writer();
        let existing: Option<i64> = conn
            .prepare_cached("SELECT id FROM clips WHERE content_hash = ?1")?
            .query_row([&clip.content_hash], |row| row.get(0))
            .optional()?;
        let store_formats = existing.is_none() || clip.kind == ClipKind::RichText;

        // Blob files go first. A crash before the commit leaves an orphan file, never a row
        // that points at a missing file.
        let mut blob_hashes = Vec::with_capacity(formats.len());
        for f in &formats {
            let hash = if store_formats && f.data.len() > MAX_INLINE_BYTES {
                Some(self.blobs.put(&f.data)?)
            } else {
                None
            };
            blob_hashes.push(hash);
        }

        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (id, uuid): (i64, String) = tx
            .prepare_cached(
                "INSERT INTO clips (uuid, kind, plain_text, content_hash, size_bytes,
                                    source_app_name, source_app_id, created_at, last_used_at, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9)
                 ON CONFLICT (content_hash) DO UPDATE SET
                   size_bytes = excluded.size_bytes,
                   source_app_name = excluded.source_app_name,
                   source_app_id = excluded.source_app_id,
                   last_used_at = excluded.last_used_at,
                   use_count = use_count + 1,
                   metadata = excluded.metadata
                 RETURNING id, uuid",
            )?
            .query_row(
                params![
                    new_uuid,
                    clip.kind.as_str(),
                    plain_text,
                    clip.content_hash,
                    size_bytes as i64,
                    clip.source_app_name,
                    clip.source_app_id,
                    now,
                    metadata,
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
        let inserted = uuid == new_uuid;

        let mut replaced_blobs = Vec::new();
        if store_formats {
            if !inserted {
                replaced_blobs = tx
                    .prepare_cached(
                        "SELECT blob_hash FROM clip_formats WHERE clip_id = ?1 AND blob_hash IS NOT NULL",
                    )?
                    .query_map([id], |row| row.get::<_, String>(0))?
                    .collect::<Result<_, _>>()?;
                tx.execute("DELETE FROM clip_formats WHERE clip_id = ?1", [id])?;
            }
            let mut insert = tx.prepare_cached(
                "INSERT INTO clip_formats (clip_id, item_index, platform, format, data, blob_hash, size_bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for (f, blob_hash) in formats.iter().zip(&blob_hashes) {
                let data = blob_hash.is_none().then_some(&f.data[..]);
                insert.execute(params![
                    id,
                    f.item_index,
                    clip.platform.as_str(),
                    f.format,
                    data,
                    blob_hash,
                    f.data.len() as i64,
                ])?;
            }
        }
        tx.commit()?;
        remove_unreferenced_blobs(&conn, &self.blobs, replaced_blobs)?;
        drop(conn);

        if inserted && clip.kind == ClipKind::Image {
            let source = formats
                .iter()
                .zip(&blob_hashes)
                .filter_map(|(f, blob_hash)| {
                    let format = SourceFormat::from_format_name(&f.format)?;
                    let source = match blob_hash {
                        Some(hash) => Source::File(self.blobs.path(hash).ok()?),
                        None => Source::Bytes(f.data.clone()),
                    };
                    Some((format, source))
                });
            self.queue_thumbnail(&clip.content_hash, source);
        }

        Ok(Upserted { id, uuid, inserted })
    }

    /// Moves a clip to the top after it's pasted. Returns `false` if there's no such clip.
    pub fn mark_used(&self, id: i64, now: i64) -> StorageResult<bool> {
        let changed = self
            .writer()
            .prepare_cached(
                "UPDATE clips SET last_used_at = ?2, use_count = use_count + 1 WHERE id = ?1",
            )?
            .execute(params![id, now])?;
        Ok(changed > 0)
    }

    pub fn get_clip(&self, id: i64) -> StorageResult<Option<Clip>> {
        self.with_reader(|conn| {
            let row = conn
                .prepare_cached(
                    "SELECT id, uuid, kind, plain_text, content_hash, size_bytes, source_app_name,
                            source_app_id, created_at, last_used_at, use_count, pinned, metadata
                     FROM clips WHERE id = ?1",
                )?
                .query_row([id], |row| {
                    Ok((
                        Clip {
                            id: row.get(0)?,
                            uuid: row.get(1)?,
                            kind: ClipKind::Text,
                            plain_text: row.get(3)?,
                            content_hash: row.get(4)?,
                            size_bytes: row.get(5)?,
                            source_app_name: row.get(6)?,
                            source_app_id: row.get(7)?,
                            created_at: row.get(8)?,
                            last_used_at: row.get(9)?,
                            use_count: row.get(10)?,
                            pinned: row.get(11)?,
                            metadata: None,
                        },
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(12)?,
                    ))
                })
                .optional()?;
            let Some((mut clip, kind, metadata)) = row else {
                return Ok(None);
            };
            clip.kind = kind.parse()?;
            clip.metadata = metadata
                .map(|m| serde_json::from_str(&m))
                .transpose()
                .map_err(|e| StorageError::Corrupt(format!("metadata of clip {id}: {e}")))?;
            Ok(Some(clip))
        })
    }

    /// The stored formats of a clip, with blob data read from disk. A plain-text format that
    /// was a copy of `plain_text` isn't among them.
    pub fn clip_formats(&self, clip_id: i64) -> StorageResult<Vec<ClipFormat>> {
        let rows = self.with_reader(|conn| {
            let rows = conn
                .prepare_cached(
                    "SELECT item_index, platform, format, data, blob_hash FROM clip_formats
                     WHERE clip_id = ?1 ORDER BY item_index, id",
                )?
                .query_map([clip_id], |row| {
                    Ok((
                        row.get::<_, u32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<Vec<u8>>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })?;
        rows.into_iter()
            .map(|(item_index, platform, format, data, blob_hash)| {
                let data = match (data, blob_hash) {
                    (Some(data), _) => data,
                    (None, Some(hash)) => self.blobs.read(&hash)?,
                    (None, None) => {
                        return Err(StorageError::Corrupt(format!(
                            "format {format:?} has no data"
                        )));
                    }
                };
                Ok(ClipFormat {
                    item_index,
                    platform: platform.parse()?,
                    format,
                    data,
                })
            })
            .collect()
    }

    /// The thumbnail for an image clip, if it has been made.
    pub fn thumbnail_path(&self, content_hash: &str) -> StorageResult<Option<PathBuf>> {
        blobs::check_hash(content_hash)?;
        let path = thumbs::thumbnail_path(&self.thumbs_dir, content_hash);
        Ok(path.is_file().then_some(path))
    }

    /// Queues a thumbnail for an image clip again, for example when the file is missing after
    /// a crash. Returns `false` if the clip isn't an image or has no format we can decode.
    pub fn request_thumbnail(&self, clip_id: i64) -> StorageResult<bool> {
        let Some(clip) = self.get_clip(clip_id)? else {
            return Ok(false);
        };
        if clip.kind != ClipKind::Image {
            return Ok(false);
        }
        let formats = self.clip_formats(clip_id)?;
        let source = formats.into_iter().filter_map(|f| {
            Some((
                SourceFormat::from_format_name(&f.format)?,
                Source::Bytes(f.data),
            ))
        });
        Ok(self.queue_thumbnail(&clip.content_hash, source))
    }

    /// Runs `f` with a read-only connection from the pool.
    pub fn with_reader<T>(
        &self,
        f: impl FnOnce(&Connection) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let idle = self
            .readers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop();
        let conn = match idle {
            Some(conn) => conn,
            None => open_reader(&self.db_path)?,
        };
        let result = f(&conn);
        let mut readers = self.readers.lock().unwrap_or_else(PoisonError::into_inner);
        if readers.len() < MAX_IDLE_READERS {
            readers.push(conn);
        }
        result
    }

    fn writer(&self) -> MutexGuard<'_, Connection> {
        self.writer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queues the best source (PNG, then TIFF, then DIB). Returns `false` if there's none.
    fn queue_thumbnail(
        &self,
        content_hash: &str,
        sources: impl Iterator<Item = (SourceFormat, Source)>,
    ) -> bool {
        let Some((format, source)) = sources.min_by_key(|(format, _)| *format as u8) else {
            return false;
        };
        self.thumbnailer.queue(thumbs::Job {
            content_hash: content_hash.to_owned(),
            format,
            source,
        });
        true
    }
}

fn configure_writer(conn: &Connection) -> StorageResult<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    // Only takes effect before the first table is created.
    conn.pragma_update(None, "page_size", 8192)?;
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(StorageError::Corrupt(format!(
            "journal_mode is {mode}, not WAL"
        )));
    }
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "secure_delete", "ON")?;
    Ok(())
}

fn open_reader(path: &Path) -> StorageResult<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(conn)
}

/// Deletes the blob files in `hashes` that no row references any more. Call it after the
/// commit, with the writer held, so no other insert can start using a file in between.
fn remove_unreferenced_blobs(
    conn: &Connection,
    blobs: &BlobStore,
    hashes: Vec<String>,
) -> StorageResult<()> {
    if hashes.is_empty() {
        return Ok(());
    }
    let mut referenced =
        conn.prepare_cached("SELECT EXISTS (SELECT 1 FROM clip_formats WHERE blob_hash = ?1)")?;
    for hash in hashes {
        if !referenced.query_row([&hash], |row| row.get::<_, bool>(0))? {
            blobs.remove(&hash)?;
        }
    }
    Ok(())
}

/// Creates the data folder so only the current user can read it.
fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Cuts `text` to [`MAX_TEXT_INDEX_BYTES`] at a character boundary.
fn truncate_text(mut text: String) -> (String, bool) {
    if text.len() <= MAX_TEXT_INDEX_BYTES {
        return (text, false);
    }
    let mut end = MAX_TEXT_INDEX_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    (text, true)
}

/// The current time in Unix milliseconds.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_at_a_char_boundary() {
        let text = "a".repeat(MAX_TEXT_INDEX_BYTES - 1) + "é";
        let (cut, truncated) = truncate_text(text);
        assert!(truncated);
        assert_eq!(cut.len(), MAX_TEXT_INDEX_BYTES - 1);

        let (same, truncated) = truncate_text("short".into());
        assert!(!truncated);
        assert_eq!(same, "short");
    }
}
