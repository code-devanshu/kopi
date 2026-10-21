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
pub mod hlc;
mod migrate;
mod model;
mod pinboards;
mod retention;
mod settings;
mod snippets;
pub mod sync;
mod thumbs;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};

pub use blobs::BlobStore;
pub use migrate::SCHEMA_VERSION;
pub use model::{
    Clip, ClipFormat, ClipKind, ExcludedApp, NewClip, NewFormat, Pinboard, Platform, Snippet,
    Upserted,
};
pub use pinboards::{KEPT, MAX_PINBOARD_NAME_CHARS};
pub use retention::{DEFAULT_MAX_TOTAL_BYTES, Limits, TOMBSTONE_DAYS};
pub use snippets::{
    MAX_SNIPPET_BODY_BYTES, MAX_SNIPPET_KEYWORD_CHARS, MAX_SNIPPET_NAME_CHARS, SnippetEdit,
};
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
    /// The caller asked for something the store doesn't allow, such as a pinboard without a
    /// name. The message is fit to show the user.
    #[error("{0}")]
    Invalid(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

/// Called after a write that gave a synced row a new `sync_seq`, so sync sends it.
pub type ChangeListener = Box<dyn Fn() + Send + Sync + 'static>;

pub struct Store {
    db_path: PathBuf,
    writer: Mutex<Connection>,
    readers: Mutex<Vec<Connection>>,
    blobs: BlobStore,
    thumbs_dir: PathBuf,
    thumbnailer: Thumbnailer,
    clock: hlc::Clock,
    change_listener: RwLock<Option<ChangeListener>>,
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
        let last_hlc = sync::last_hlc(&writer)?;

        Ok(Self {
            db_path,
            writer: Mutex::new(writer),
            readers: Mutex::new(Vec::new()),
            blobs: BlobStore::new(blobs_dir),
            thumbnailer: Thumbnailer::spawn(thumbs_dir.clone(), listener)?,
            thumbs_dir,
            clock: hlc::Clock::new(last_hlc),
            change_listener: RwLock::new(None),
        })
    }

    /// Sets the function called after each write that changes something sync sends.
    pub fn set_change_listener(&self, listener: ChangeListener) {
        *self
            .change_listener
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(listener);
    }

    /// Tells the change listener, if there is one.
    fn changed(&self) {
        if let Some(listener) = self
            .change_listener
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            listener();
        }
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
    /// - Either way the clip gets a new HLC and `sync_seq`, and counts as copied here.
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
        let seq = next_seq(&tx)?;
        let (id, uuid): (i64, String) = tx
            .prepare_cached(
                "INSERT INTO clips (uuid, kind, plain_text, content_hash, size_bytes,
                                    source_app_name, source_app_id, created_at, last_used_at, metadata,
                                    used_hlc, sync_seq, local_only)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT (content_hash) DO UPDATE SET
                   size_bytes = excluded.size_bytes,
                   source_app_name = excluded.source_app_name,
                   source_app_id = excluded.source_app_id,
                   last_used_at = excluded.last_used_at,
                   use_count = use_count + 1,
                   metadata = excluded.metadata,
                   used_hlc = excluded.used_hlc,
                   sync_seq = excluded.sync_seq,
                   copied_on = NULL,
                   local_only = max(local_only, excluded.local_only)
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
                    self.clock.tick(now),
                    seq,
                    clip.local_only,
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
        self.changed();

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
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx
            .prepare_cached(
                "UPDATE clips SET last_used_at = ?2, use_count = use_count + 1, used_hlc = ?3,
                                  sync_seq = ?4
                 WHERE id = ?1",
            )?
            .execute(params![id, now, self.clock.tick(now), next_seq(&tx)?])?;
        if changed == 0 {
            return Ok(false);
        }
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(true)
    }

    /// Deletes clips, pinned or not, with their formats, blob files and thumbnails. Returns the
    /// ids that were found and deleted, in the order given. Each leaves a tombstone, so the
    /// delete reaches the other computers.
    pub fn delete_clips(&self, ids: &[i64]) -> StorageResult<Vec<i64>> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let removed = delete_in(&tx, ids.iter().copied())?;
        let hlc = self.clock.tick(now_ms());
        for hash in &removed.content_hashes {
            put_tombstone(&tx, Tombstone::CLIP, hash, hlc)?;
        }
        tx.commit()?;
        let ids = self.remove_files(conn, removed)?;
        if !ids.is_empty() {
            self.changed();
        }
        Ok(ids)
    }

    pub fn get_clip(&self, id: i64) -> StorageResult<Option<Clip>> {
        self.with_reader(|conn| {
            let row = conn
                .prepare_cached(
                    "SELECT id, uuid, kind, plain_text, content_hash, size_bytes, source_app_name,
                            source_app_id, created_at, last_used_at, use_count, pinned, metadata,
                            (SELECT d.name FROM sync_devices d WHERE d.device_id = copied_on)
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
                            copied_on: row.get(13)?,
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

    /// The clip with this content hash, if there is one.
    pub fn clip_id_by_hash(&self, content_hash: &str) -> StorageResult<Option<i64>> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached("SELECT id FROM clips WHERE content_hash = ?1")?
                .query_row([content_hash], |row| row.get(0))
                .optional()?)
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

    /// The apps excluded on `platform`, by name.
    pub fn excluded_apps(&self, platform: Platform) -> StorageResult<Vec<ExcludedApp>> {
        self.with_reader(|conn| {
            let rows = conn
                .prepare_cached(
                    "SELECT app_id, app_name, origin, created_at FROM excluded_apps
                     WHERE platform = ?1 ORDER BY app_name COLLATE NOCASE, app_id",
                )?
                .query_map([platform.as_str()], |row| {
                    Ok(ExcludedApp {
                        platform,
                        app_id: row.get(0)?,
                        app_name: row.get(1)?,
                        is_default: row.get::<_, String>(2)? == "default",
                        created_at: row.get(3)?,
                    })
                })?
                .collect::<Result<_, _>>()?;
            Ok(rows)
        })
    }

    /// Excludes an app the user picked. Returns `false` if it was already excluded. IDs are
    /// compared without regard to case, as macOS bundle IDs and Windows paths are.
    pub fn add_excluded_app(
        &self,
        platform: Platform,
        app_id: &str,
        app_name: &str,
    ) -> StorageResult<bool> {
        let added = self
            .writer()
            .prepare_cached(
                "INSERT INTO excluded_apps (platform, app_id, app_name, origin, created_at)
                 SELECT ?1, ?2, ?3, 'user', ?4
                 WHERE NOT EXISTS (SELECT 1 FROM excluded_apps
                                   WHERE platform = ?1 AND app_id = ?2 COLLATE NOCASE)",
            )?
            .execute(params![platform.as_str(), app_id, app_name, now_ms()])?;
        Ok(added > 0)
    }

    /// Stops excluding an app, a default or one the user added. A removed default isn't seeded
    /// again. Returns `false` if it wasn't excluded.
    pub fn remove_excluded_app(&self, platform: Platform, app_id: &str) -> StorageResult<bool> {
        let removed = self
            .writer()
            .prepare_cached(
                "DELETE FROM excluded_apps WHERE platform = ?1 AND app_id = ?2 COLLATE NOCASE",
            )?
            .execute(params![platform.as_str(), app_id])?;
        Ok(removed > 0)
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

    /// Removes the files of clips deleted by a committed transaction, and returns their ids.
    /// Blob files go first, with the writer still held, then the thumbnails without it.
    fn remove_files(
        &self,
        conn: MutexGuard<'_, Connection>,
        removed: Removed,
    ) -> StorageResult<Vec<i64>> {
        remove_unreferenced_blobs(&conn, &self.blobs, removed.blob_hashes)?;
        drop(conn);

        // Content hashes are unique, so no other clip uses these thumbnails.
        for hash in removed.content_hashes {
            if blobs::check_hash(&hash).is_err() {
                continue;
            }
            match fs::remove_file(thumbs::thumbnail_path(&self.thumbs_dir, &hash)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
        }
        Ok(removed.ids)
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

/// Clips deleted in a transaction, and the files to remove once it commits.
#[derive(Default)]
struct Removed {
    ids: Vec<i64>,
    content_hashes: Vec<String>,
    blob_hashes: Vec<String>,
}

impl Removed {
    fn extend(&mut self, other: Removed) {
        self.ids.extend(other.ids);
        self.content_hashes.extend(other.content_hashes);
        self.blob_hashes.extend(other.blob_hashes);
    }
}

/// Deletes clips in `tx`, skipping ids that don't exist. Their formats, pinboard places and
/// search entries go with them. The files are left for [`Store::remove_files`].
fn delete_in(tx: &Connection, ids: impl IntoIterator<Item = i64>) -> StorageResult<Removed> {
    let mut removed = Removed::default();
    let mut blobs = tx.prepare_cached(
        "SELECT blob_hash FROM clip_formats WHERE clip_id = ?1 AND blob_hash IS NOT NULL",
    )?;
    let mut delete = tx.prepare_cached("DELETE FROM clips WHERE id = ?1 RETURNING content_hash")?;
    for id in ids {
        let clip_blobs = blobs
            .query_map([id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let Some(hash) = delete
            .query_row([id], |row| row.get::<_, String>(0))
            .optional()?
        else {
            continue;
        };
        removed.ids.push(id);
        removed.content_hashes.push(hash);
        removed.blob_hashes.extend(clip_blobs);
    }
    Ok(removed)
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
pub(crate) fn truncate_text(mut text: String) -> (String, bool) {
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

/// Gives out the next `sync_seq`. Call it inside the write's transaction, so numbers are
/// committed in order.
pub(crate) fn next_seq(conn: &Connection) -> rusqlite::Result<i64> {
    conn.prepare_cached(
        "UPDATE settings SET value = CAST(value AS INTEGER) + 1 WHERE key = 'sync.seq'
         RETURNING CAST(value AS INTEGER)",
    )?
    .query_row([], |row| row.get(0))
}

/// The kinds of thing a tombstone can stand for.
pub(crate) struct Tombstone;

impl Tombstone {
    pub(crate) const CLIP: &'static str = "clip";
    pub(crate) const PINBOARD: &'static str = "pinboard";
    pub(crate) const PINBOARD_ITEM: &'static str = "pinboard_item";
    pub(crate) const SNIPPET: &'static str = "snippet";

    /// The key of a pinboard item's tombstone.
    pub(crate) fn item_key(pinboard_uuid: &str, content_hash: &str) -> String {
        format!("{pinboard_uuid}/{content_hash}")
    }
}

/// Records a delete, or moves an older tombstone for the same thing forward. Returns whether
/// it changed anything.
pub(crate) fn put_tombstone(
    conn: &Connection,
    entity: &str,
    key: &str,
    hlc: i64,
) -> rusqlite::Result<bool> {
    let current: Option<i64> = conn
        .prepare_cached("SELECT hlc FROM tombstones WHERE entity = ?1 AND key = ?2")?
        .query_row(params![entity, key], |row| row.get(0))
        .optional()?;
    if current.is_some_and(|current| current >= hlc) {
        return Ok(false);
    }
    conn.prepare_cached(
        "INSERT INTO tombstones (entity, key, hlc, sync_seq) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (entity, key) DO UPDATE SET hlc = excluded.hlc, sync_seq = excluded.sync_seq",
    )?
    .execute(params![entity, key, hlc, next_seq(conn)?])?;
    Ok(true)
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
