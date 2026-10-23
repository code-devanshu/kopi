//! The text read from image clips, kept in their `plain_text` so search finds it. The reading
//! itself is in [`crate::ocr`]. See `docs/OCR_DESIGN.md`.
//!
//! An image's `ocr_version` is NULL until it's been read, then the version of the reader that
//! read it. Its `size_bytes` counts the text, so the history limits do too. The text isn't
//! synced, so none of this gives a clip a new `sync_seq`.
//!
//! The backlog's queries name the partial index `clips_ocr_pending`. SQLite otherwise picks
//! `clips_by_kind` and walks past every image that's been read.

use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::Value;

use super::{ClipFormat, StorageError, StorageResult, Store, now_ms};

/// The setting that turns reading on and off: `true` or `false`, on by default.
const IMAGE_TEXT: &str = "capture.image_text";

/// The version of the reader. A change that reads better bumps it, with a migration that sets
/// older versions back to NULL, so the backlog reads those images again. They keep their old
/// text until then.
pub const IMAGE_TEXT_READER: u32 = 1;

/// Called after an image clip is inserted, by a copy or by sync, with its id.
pub type ImageListener = Box<dyn Fn(i64) + Send + Sync + 'static>;

impl Store {
    /// Sets the function called after each image clip is inserted.
    pub fn set_image_listener(&self, listener: ImageListener) {
        *self
            .image_listener
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(listener);
    }

    /// Tells the image listener, if there is one.
    pub(super) fn image_added(&self, id: i64) {
        if let Some(listener) = self
            .image_listener
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            listener(id);
        }
    }

    /// Whether Kopi reads the text in images. On by default.
    pub fn image_text_enabled(&self) -> StorageResult<bool> {
        Ok(self
            .setting(IMAGE_TEXT)?
            .and_then(|v| v.as_bool())
            .unwrap_or(true))
    }

    /// Turns reading on or off. Turning it off also removes the text read from every image,
    /// in the same transaction, so search stops finding them and turning it on again reads
    /// them all again.
    pub fn set_image_text_enabled(&self, on: bool) -> StorageResult<()> {
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.prepare_cached(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
             WHERE value IS NOT excluded.value",
        )?
        .execute(params![IMAGE_TEXT, Value::from(on).to_string(), now_ms()])?;
        if !on {
            // SET reads the row as it was, so size_bytes loses the old text's length.
            tx.execute(
                "UPDATE clips SET size_bytes = size_bytes - coalesce(octet_length(plain_text), 0),
                                  plain_text = NULL, ocr_version = NULL
                 WHERE kind = 'image' AND (plain_text IS NOT NULL OR ocr_version IS NOT NULL)",
                [],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// How many images are waiting to be read.
    pub fn images_without_text(&self) -> StorageResult<i64> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(
                    "SELECT count(*) FROM clips INDEXED BY clips_ocr_pending
                     WHERE kind = 'image' AND ocr_version IS NULL",
                )?
                .query_row([], |row| row.get(0))?)
        })
    }

    /// The most recently used image that's waiting to be read.
    pub fn next_image_without_text(&self) -> StorageResult<Option<i64>> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(
                    "SELECT id FROM clips INDEXED BY clips_ocr_pending
                     WHERE kind = 'image' AND ocr_version IS NULL
                     ORDER BY last_used_at DESC LIMIT 1",
                )?
                .query_row([], |row| row.get(0))
                .optional()?)
        })
    }

    /// The formats of image `id`, while it's waiting to be read. None if it isn't an image, has
    /// been read, or was deleted, even while its formats were being read.
    pub(crate) fn unread_image(&self, id: i64) -> StorageResult<Option<Vec<ClipFormat>>> {
        let unread = self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(
                    "SELECT EXISTS (SELECT 1 FROM clips
                                    WHERE id = ?1 AND kind = 'image' AND ocr_version IS NULL)",
                )?
                .query_row([id], |row| row.get::<_, bool>(0))?)
        })?;
        if !unread {
            return Ok(None);
        }
        match self.clip_formats(id) {
            Ok(formats) => Ok(Some(formats)),
            // A blob removed after the check: the clip was just deleted.
            Err(StorageError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Stores the text read from image `id` by version `reader` of the reader, or `None` when
    /// it has none. Only while the image is waiting and reading is on, so a clip deleted while
    /// it was read, or text read just after the setting was turned off, isn't written. The
    /// search index follows by trigger. Returns whether it was written.
    pub fn write_image_text(
        &self,
        id: i64,
        text: Option<&str>,
        reader: u32,
    ) -> StorageResult<bool> {
        let written = self
            .writer()
            .prepare_cached(
                "UPDATE clips SET plain_text = ?2, ocr_version = ?3,
                                  size_bytes = size_bytes - coalesce(octet_length(plain_text), 0)
                                               + coalesce(octet_length(?2), 0)
                 WHERE id = ?1 AND kind = 'image' AND ocr_version IS NULL
                   AND NOT EXISTS (SELECT 1 FROM settings WHERE key = ?4 AND value = 'false')",
            )?
            .execute(params![id, text, reader, IMAGE_TEXT])?;
        Ok(written > 0)
    }
}
