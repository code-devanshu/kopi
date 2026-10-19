//! Sync's side of the store: the group of computers, the changes to send, and merging the
//! changes another computer sent. The network is in `kopi-sync`. See `docs/SYNC_DESIGN.md`.
//!
//! Every change to a synced row gives it the next `sync_seq`. Sending is "every row with a
//! `sync_seq` above what the other computer has", in batches. Merging follows the rules in the
//! design, which give the same result in any order and any number of times:
//!
//! - **Clips** are one per `content_hash`. The latest use wins `last_used_at`, the source app
//!   and the computer it was copied on. The earliest `created_at` wins. The latest pin or unpin
//!   wins. `use_count` stays local.
//! - **Pinboards** and **pinboard items**: the latest change wins the whole row.
//! - **Tombstones** delete what's older than them, and keep it from coming back.
//! - **Cleared ranges** delete the clips last used in their window before they were made.
//! - **Devices**: the latest change wins. A device's public key never changes.
//!
//! On a tie in HLCs, the larger value wins, so every computer picks the same one. A tombstone
//! wins a tie with what it deletes.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use kopi_platform::uti;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use super::pinboards::{check_color, check_name};
use super::{
    Clip, ClipKind, KEPT, MAX_INLINE_BYTES, NewFormat, Platform, Removed, StorageError,
    StorageResult, Store, Tombstone, blobs, delete_in, next_seq, now_ms, put_tombstone,
    remove_unreferenced_blobs,
};
use crate::paste::{Content, clipboard_formats};
use crate::privacy::DEFAULT_MAX_CLIP_BYTES;

/// The most rows in one batch.
pub const MAX_BATCH_ROWS: usize = 256;
/// A batch stops growing once its clips hold this much. A larger clip goes alone.
pub const MAX_BATCH_BYTES: usize = 8 * 1024 * 1024;
/// The largest clip accepted from another computer: the capture limit, plus the plain text
/// that's added back for sending.
pub const MAX_RECEIVED_CLIP_BYTES: u64 =
    DEFAULT_MAX_CLIP_BYTES + 2 * super::MAX_TEXT_INDEX_BYTES as u64;

const MAX_FORMATS: usize = 64;
const MAX_FORMAT_NAME_BYTES: usize = 256;
const MAX_ITEM_INDEX: u32 = 4096;
const MAX_METADATA_BYTES: usize = 16 * 1024;
const MAX_APP_BYTES: usize = 1024;
const MAX_ID_BYTES: usize = 64;
const MAX_KEY_BYTES: usize = 200;
/// The longest computer name, in characters.
pub const MAX_DEVICE_NAME_CHARS: usize = 64;

/// One batch of changes, as sent between computers. Fields an older Kopi doesn't know are
/// skipped when it reads them, so new ones can be added.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Changes {
    pub devices: Vec<SyncDevice>,
    pub tombstones: Vec<SyncTombstone>,
    pub clear_ranges: Vec<SyncClearRange>,
    pub clips: Vec<SyncClip>,
    pub pinboards: Vec<SyncPinboard>,
    pub pinboard_items: Vec<SyncPinboardItem>,
    /// The highest `sync_seq` this batch covers. Rows up to it that aren't in it weren't for
    /// the receiver, such as local-only clips.
    pub max_seq: i64,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
            && self.tombstones.is_empty()
            && self.clear_ranges.is_empty()
            && self.clips.is_empty()
            && self.pinboards.is_empty()
            && self.pinboard_items.is_empty()
    }

    /// How many rows it holds.
    pub fn len(&self) -> usize {
        self.devices.len()
            + self.tombstones.len()
            + self.clear_ranges.len()
            + self.clips.len()
            + self.pinboards.len()
            + self.pinboard_items.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncClip {
    pub content_hash: String,
    pub kind: String,
    /// Every format, with the plain text the store leaves out put back.
    pub formats: Vec<SyncFormat>,
    /// JSON, as in `clips.metadata`.
    pub metadata: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
    pub used_hlc: i64,
    pub source_app_name: Option<String>,
    pub source_app_id: Option<String>,
    /// The device ID of the computer it was last copied on.
    pub copied_on: String,
    pub pinned: bool,
    pub pinned_hlc: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncFormat {
    pub item_index: u32,
    pub format: String,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncPinboard {
    pub uuid: String,
    pub name: String,
    pub color: Option<String>,
    pub position: i64,
    pub created_at: i64,
    pub updated_hlc: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncPinboardItem {
    /// The pinboard's uuid.
    pub pinboard: String,
    /// The clip's content hash.
    pub clip: String,
    pub position: i64,
    pub added_at: i64,
    pub hlc: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncTombstone {
    /// `clip`, `pinboard` or `pinboard_item`.
    pub entity: String,
    pub key: String,
    pub hlc: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncClearRange {
    pub from_ms: Option<i64>,
    pub to_ms: i64,
    pub include_kept: bool,
    pub hlc: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncDevice {
    pub device_id: String,
    #[serde(with = "serde_bytes")]
    pub public_key: Vec<u8>,
    pub name: String,
    /// `macos` or `windows`.
    pub os: String,
    pub removed: bool,
    pub updated_hlc: i64,
}

/// A row of `sync_devices`: a computer in the sync group, or this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub device_id: String,
    pub public_key: Vec<u8>,
    pub name: String,
    pub os: Platform,
    /// Removed from the group. Its key is no longer accepted.
    pub removed: bool,
    /// The highest of its `sync_seq`s applied here.
    pub received_seq: i64,
    /// When a session with it last ran, in Unix milliseconds.
    pub last_seen_at: Option<i64>,
}

/// What [`Store::apply_changes`] changed, for the UI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    /// New clips.
    pub added: Vec<i64>,
    /// Clips that moved, were pinned or unpinned.
    pub updated: Vec<i64>,
    /// Clips deleted by tombstones or cleared ranges.
    pub removed: Vec<i64>,
    pub pinboards_changed: bool,
    pub devices_changed: bool,
    /// Clips dropped: a bad hash, too large, or invalid. Counted for the log.
    pub rejected: usize,
}

impl Applied {
    /// Whether anything the UI shows changed.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.updated.is_empty()
            && self.removed.is_empty()
            && !self.pinboards_changed
            && !self.devices_changed
    }
}

/// The highest HLC stored anywhere, so the clock continues after it.
pub(crate) fn last_hlc(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT max(coalesce((SELECT max(used_hlc) FROM clips), 0),
                    coalesce((SELECT max(pinned_hlc) FROM clips), 0),
                    coalesce((SELECT max(updated_hlc) FROM pinboards), 0),
                    coalesce((SELECT max(hlc) FROM pinboard_items), 0),
                    coalesce((SELECT max(hlc) FROM tombstones), 0),
                    coalesce((SELECT max(hlc) FROM clear_ranges), 0),
                    coalesce((SELECT max(updated_hlc) FROM sync_devices), 0))",
        [],
        |row| row.get(0),
    )
}

/// A row with a `sync_seq` above the other computer's cursor.
enum Pending {
    Clip(i64),
    Pinboard(i64),
    Item { pinboard_id: i64, clip_id: i64 },
    Tombstone { entity: String, key: String },
    ClearRange(i64),
    Device(String),
}

/// A batch being put together. A clip comes with the pinboard items it's on, and an item with
/// its clip and its pinboard, so the receiver can always place what it gets. That matters when
/// a clip deleted on one computer comes back from another: its items come back with it.
struct Batch<'a> {
    store: &'a Store,
    conn: &'a Connection,
    kinds: crate::privacy::KindSet,
    self_id: &'a str,
    changes: Changes,
    bytes: usize,
    /// Clips looked at, and whether they're in the batch.
    clips: HashMap<i64, bool>,
    boards: HashSet<i64>,
    items: HashSet<(i64, i64)>,
}

impl Batch<'_> {
    /// Adds a clip and its items, unless it isn't for the receiver. Returns whether it's in.
    fn clip(&mut self, id: i64) -> StorageResult<bool> {
        if let Some(&sent) = self.clips.get(&id) {
            return Ok(sent);
        }
        let clip = self
            .store
            .read_clip(self.conn, id, self.kinds, self.self_id)?;
        let sent = clip.is_some();
        self.clips.insert(id, sent);
        if let Some(clip) = clip {
            self.bytes += clip_bytes(&clip);
            self.changes.clips.push(clip);
            let boards: Vec<i64> = self
                .conn
                .prepare_cached("SELECT pinboard_id FROM pinboard_items WHERE clip_id = ?1")?
                .query_map([id], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            for board in boards {
                self.item(board, id)?;
            }
        }
        Ok(sent)
    }

    fn board(&mut self, id: i64) -> StorageResult<()> {
        if self.boards.insert(id)
            && let Some(board) = read_pinboard(self.conn, id)?
        {
            self.changes.pinboards.push(board);
        }
        Ok(())
    }

    /// Adds an item with its clip and pinboard. Skipped if its clip isn't for the receiver.
    fn item(&mut self, pinboard_id: i64, clip_id: i64) -> StorageResult<()> {
        if self.items.contains(&(pinboard_id, clip_id)) || !self.clip(clip_id)? {
            return Ok(());
        }
        let Some(item) = read_item(self.conn, pinboard_id, clip_id)? else {
            return Ok(());
        };
        self.items.insert((pinboard_id, clip_id));
        self.board(pinboard_id)?;
        self.changes.pinboard_items.push(item);
        Ok(())
    }
}

/// A received clip that passed its checks.
struct Checked {
    kind: ClipKind,
    content_hash: String,
    plain_text: Option<String>,
    formats: Vec<NewFormat>,
    metadata: Option<String>,
    created_at: i64,
    last_used_at: i64,
    used_hlc: i64,
    source_app_name: Option<String>,
    source_app_id: Option<String>,
    copied_on: String,
    pinned: bool,
    pinned_hlc: i64,
}

/// What a clip's last use is compared by: its HLC, then its values.
type UseKey = (i64, i64, Option<String>, Option<String>, Option<String>);

impl Store {
    /// Every computer in the group, this one included, removed ones too, by name.
    pub fn sync_devices(&self) -> StorageResult<Vec<Device>> {
        self.with_reader(|conn| {
            let rows = conn
                .prepare_cached(
                    "SELECT device_id, public_key, name, os, removed, received_seq, last_seen_at
                     FROM sync_devices ORDER BY name COLLATE NOCASE, device_id",
                )?
                .query_map([], |row| {
                    Ok((
                        Device {
                            device_id: row.get(0)?,
                            public_key: row.get(1)?,
                            name: row.get(2)?,
                            os: Platform::MacOs,
                            removed: row.get(4)?,
                            received_seq: row.get(5)?,
                            last_seen_at: row.get(6)?,
                        },
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows.into_iter()
                .map(|(mut device, os)| {
                    device.os = os.parse()?;
                    Ok(device)
                })
                .collect()
        })
    }

    /// Adds this computer to the group, or updates its name. Changing the key of an existing
    /// device ID replaces it.
    pub fn set_self_device(
        &self,
        device_id: &str,
        public_key: &[u8],
        name: &str,
        os: Platform,
    ) -> StorageResult<()> {
        let name = check_device_name(name)?;
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<(Vec<u8>, String, String, bool)> = tx
            .prepare_cached(
                "SELECT public_key, name, os, removed FROM sync_devices WHERE device_id = ?1",
            )?
            .query_row([device_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .optional()?;
        let same = current.is_some_and(|(key, current_name, current_os, removed)| {
            key == public_key && current_name == name && current_os == os.as_str() && !removed
        });
        if same {
            return Ok(());
        }
        upsert_device(
            &tx,
            device_id,
            public_key,
            &name,
            os,
            false,
            self.clock.tick(now),
        )?;
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(())
    }

    /// Adds a computer that was just paired, or brings back one that was removed.
    pub fn pair_device(
        &self,
        device_id: &str,
        public_key: &[u8],
        name: &str,
        os: Platform,
    ) -> StorageResult<()> {
        check_id(device_id)
            .then_some(())
            .ok_or_else(|| StorageError::Invalid("not a device ID".into()))?;
        if public_key.len() != 32 {
            return Err(StorageError::Invalid("not a public key".into()));
        }
        let name = check_device_name(name)?;
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // A key belongs to one device ID. An old row with this key is the same computer.
        tx.prepare_cached("DELETE FROM sync_devices WHERE public_key = ?1 AND device_id != ?2")?
            .execute(params![public_key, device_id])?;
        upsert_device(
            &tx,
            device_id,
            public_key,
            &name,
            os,
            false,
            self.clock.tick(now),
        )?;
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(())
    }

    /// Removes a computer from the group, on every computer in it. Returns `false` if it
    /// wasn't in the group.
    pub fn remove_device(&self, device_id: &str) -> StorageResult<bool> {
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx
            .prepare_cached(
                "UPDATE sync_devices SET removed = 1, updated_hlc = ?2, sync_seq = ?3
                 WHERE device_id = ?1 AND removed = 0",
            )?
            .execute(params![device_id, self.clock.tick(now), next_seq(&tx)?])?;
        if changed == 0 {
            return Ok(false);
        }
        tx.commit()?;
        drop(conn);
        self.changed();
        Ok(true)
    }

    /// Notes that a session with a computer ran just now.
    pub fn device_seen(&self, device_id: &str, now: i64) -> StorageResult<()> {
        self.writer()
            .prepare_cached("UPDATE sync_devices SET last_seen_at = ?2 WHERE device_id = ?1")?
            .execute(params![device_id, now])?;
        Ok(())
    }

    /// The highest `sync_seq` given out so far.
    pub fn sync_seq(&self) -> StorageResult<i64> {
        self.with_reader(|conn| {
            Ok(conn
                .prepare_cached(
                    "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'sync.seq'",
                )?
                .query_row([], |row| row.get(0))
                .optional()?
                .unwrap_or(0))
        })
    }

    /// The next batch of changes with a `sync_seq` above `after`, for a computer that wants
    /// `kinds`. `self_id` is this computer's device ID. An empty batch with `max_seq` equal to
    /// `after` means there's nothing more.
    ///
    /// Local-only clips, file clips and kinds the other computer doesn't want are left out.
    /// A pinboard item comes with its clip and its pinboard, so the receiver can place it.
    pub fn changes_since(
        &self,
        after: i64,
        kinds: crate::privacy::KindSet,
        self_id: &str,
    ) -> StorageResult<Changes> {
        self.with_reader(|conn| {
            // One snapshot for the whole batch.
            conn.execute_batch("BEGIN")?;
            let result = self.collect_changes(conn, after, kinds, self_id);
            let _ = conn.execute_batch("COMMIT");
            result
        })
    }

    fn collect_changes(
        &self,
        conn: &Connection,
        after: i64,
        kinds: crate::privacy::KindSet,
        self_id: &str,
    ) -> StorageResult<Changes> {
        let limit = MAX_BATCH_ROWS as i64;
        let mut pending: Vec<(i64, Pending)> = Vec::new();
        let mut seqs =
            |sql: &str, map: &dyn Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Pending>| {
                let rows = conn
                    .prepare_cached(sql)?
                    .query_map(params![after, limit], |row| Ok((row.get(0)?, map(row)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                pending.extend(rows);
                Ok::<_, StorageError>(())
            };
        seqs(
            "SELECT sync_seq, id FROM clips WHERE sync_seq > ?1 ORDER BY sync_seq LIMIT ?2",
            &|row| Ok(Pending::Clip(row.get(1)?)),
        )?;
        seqs(
            "SELECT sync_seq, id FROM pinboards WHERE sync_seq > ?1 ORDER BY sync_seq LIMIT ?2",
            &|row| Ok(Pending::Pinboard(row.get(1)?)),
        )?;
        seqs(
            "SELECT sync_seq, pinboard_id, clip_id FROM pinboard_items
             WHERE sync_seq > ?1 ORDER BY sync_seq LIMIT ?2",
            &|row| {
                Ok(Pending::Item {
                    pinboard_id: row.get(1)?,
                    clip_id: row.get(2)?,
                })
            },
        )?;
        seqs(
            "SELECT sync_seq, entity, key FROM tombstones
             WHERE sync_seq > ?1 ORDER BY sync_seq LIMIT ?2",
            &|row| {
                Ok(Pending::Tombstone {
                    entity: row.get(1)?,
                    key: row.get(2)?,
                })
            },
        )?;
        seqs(
            "SELECT sync_seq, id FROM clear_ranges WHERE sync_seq > ?1 ORDER BY sync_seq LIMIT ?2",
            &|row| Ok(Pending::ClearRange(row.get(1)?)),
        )?;
        seqs(
            "SELECT sync_seq, device_id FROM sync_devices
             WHERE sync_seq > ?1 ORDER BY sync_seq LIMIT ?2",
            &|row| Ok(Pending::Device(row.get(1)?)),
        )?;
        // The first rows of each table include the first rows of all of them.
        pending.sort_by_key(|(seq, _)| *seq);
        pending.truncate(MAX_BATCH_ROWS);

        let mut batch = Batch {
            store: self,
            conn,
            kinds,
            self_id,
            changes: Changes {
                max_seq: after,
                ..Changes::default()
            },
            bytes: 0,
            clips: HashMap::new(),
            boards: HashSet::new(),
            items: HashSet::new(),
        };
        for (seq, row) in pending {
            if batch.bytes >= MAX_BATCH_BYTES {
                break;
            }
            let changes = &mut batch.changes;
            match row {
                Pending::Clip(id) => {
                    batch.clip(id)?;
                }
                Pending::Pinboard(id) => batch.board(id)?,
                Pending::Item {
                    pinboard_id,
                    clip_id,
                } => batch.item(pinboard_id, clip_id)?,
                Pending::Tombstone { entity, key } => {
                    let hlc: Option<i64> = conn
                        .prepare_cached(
                            "SELECT hlc FROM tombstones WHERE entity = ?1 AND key = ?2",
                        )?
                        .query_row(params![entity, key], |row| row.get(0))
                        .optional()?;
                    if let Some(hlc) = hlc {
                        changes.tombstones.push(SyncTombstone { entity, key, hlc });
                    }
                }
                Pending::ClearRange(id) => {
                    let range = conn
                        .prepare_cached(
                            "SELECT from_ms, to_ms, include_kept, hlc FROM clear_ranges WHERE id = ?1",
                        )?
                        .query_row([id], |row| {
                            Ok(SyncClearRange {
                                from_ms: row.get(0)?,
                                to_ms: row.get(1)?,
                                include_kept: row.get(2)?,
                                hlc: row.get(3)?,
                            })
                        })
                        .optional()?;
                    changes.clear_ranges.extend(range);
                }
                Pending::Device(id) => {
                    let device = conn
                        .prepare_cached(
                            "SELECT public_key, name, os, removed, updated_hlc FROM sync_devices
                             WHERE device_id = ?1",
                        )?
                        .query_row([&id], |row| {
                            Ok(SyncDevice {
                                device_id: id.clone(),
                                public_key: row.get(0)?,
                                name: row.get(1)?,
                                os: row.get(2)?,
                                removed: row.get(3)?,
                                updated_hlc: row.get(4)?,
                            })
                        })
                        .optional()?;
                    changes.devices.extend(device);
                }
            }
            batch.changes.max_seq = seq;
        }
        Ok(batch.changes)
    }

    /// A clip as it's sent, or `None` if it isn't for the receiver or was just deleted.
    fn read_clip(
        &self,
        conn: &Connection,
        id: i64,
        kinds: crate::privacy::KindSet,
        self_id: &str,
    ) -> StorageResult<Option<SyncClip>> {
        type Row = (
            String,
            String,
            Option<String>,
            Option<String>,
            i64,
            i64,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            bool,
            i64,
            bool,
        );
        let row: Option<Row> = conn
            .prepare_cached(
                "SELECT kind, content_hash, plain_text, metadata, created_at, last_used_at,
                        used_hlc, source_app_name, source_app_id, copied_on, pinned, pinned_hlc,
                        local_only
                 FROM clips WHERE id = ?1",
            )?
            .query_row([id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                ))
            })
            .optional()?;
        let Some((
            kind,
            content_hash,
            plain_text,
            metadata,
            created_at,
            last_used_at,
            used_hlc,
            source_app_name,
            source_app_id,
            copied_on,
            pinned,
            pinned_hlc,
            local_only,
        )) = row
        else {
            return Ok(None);
        };
        let kind: ClipKind = kind.parse()?;
        if local_only || kind == ClipKind::File || !kinds.contains(kind) {
            return Ok(None);
        }
        let stored = match self.formats_in(conn, id) {
            Ok(stored) => stored,
            // A blob removed after this snapshot: the clip was just deleted.
            Err(StorageError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        let clip = Clip {
            id,
            uuid: String::new(),
            kind,
            plain_text,
            content_hash: content_hash.clone(),
            size_bytes: 0,
            source_app_name: None,
            source_app_id: None,
            created_at,
            last_used_at,
            use_count: 0,
            pinned,
            metadata: None,
            copied_on: None,
        };
        let mut formats: Vec<SyncFormat> = clipboard_formats(&clip, &stored, Content::All)
            .into_iter()
            .map(|f| SyncFormat {
                item_index: f.item_index,
                format: f.format,
                data: f.data,
            })
            .collect();
        if kind == ClipKind::Image {
            add_png(&mut formats);
        }
        Ok(Some(SyncClip {
            content_hash,
            kind: kind.as_str().to_owned(),
            formats,
            metadata,
            created_at,
            last_used_at,
            used_hlc,
            source_app_name,
            source_app_id,
            copied_on: copied_on.unwrap_or_else(|| self_id.to_owned()),
            pinned,
            pinned_hlc,
        }))
    }

    /// A clip's stored formats, read through `conn`, with blob data from disk.
    fn formats_in(&self, conn: &Connection, clip_id: i64) -> StorageResult<Vec<super::ClipFormat>> {
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
                Ok(super::ClipFormat {
                    item_index,
                    platform: platform.parse()?,
                    format,
                    data,
                })
            })
            .collect()
    }

    /// Merges a batch from the computer `from`, in one transaction, and records that its
    /// changes up to `changes.max_seq` are applied. `self_id` is this computer's device ID.
    ///
    /// Everything is checked as untrusted input. A clip whose hash doesn't match its content,
    /// that's too large, or of a kind this computer doesn't sync, is dropped. So is a clip
    /// copied in an app this computer excludes, when it was copied on the same OS.
    pub fn apply_changes(
        &self,
        from: &str,
        self_id: &str,
        changes: Changes,
    ) -> StorageResult<Applied> {
        let now = now_ms();
        let kinds = self.sync_kinds()?;
        let os = local_platform();
        let excluded: HashSet<String> = self
            .excluded_apps(os)?
            .into_iter()
            .map(|app| app.app_id.to_lowercase())
            .collect();

        let mut applied = Applied::default();
        // Checking hashes is the slow part, so it's done before taking the writer.
        let mut checked = Vec::with_capacity(changes.clips.len());
        for clip in changes.clips {
            match check_clip(clip) {
                Some(clip) if kinds.contains(clip.kind) => checked.push(clip),
                Some(_) => {}
                None => applied.rejected += 1,
            }
        }

        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut removed = Removed::default();
        let mut written_blobs = Vec::new();
        let mut thumbnails = Vec::new();

        for device in changes.devices {
            if device.device_id != self_id && self.apply_device(&tx, device, now)? {
                applied.devices_changed = true;
            }
        }
        // Ranges first: a computer's range comes before the tombstones it left for the clips
        // the range removed there, and kept clips here must be stamped before those arrive.
        for range in changes.clear_ranges {
            self.apply_clear_range(&tx, range, now, &mut removed)?;
        }
        for tombstone in changes.tombstones {
            self.apply_tombstone(&tx, tombstone, now, &mut removed, &mut applied)?;
        }
        for clip in checked {
            if clip.copied_on != self_id
                && device_os(&tx, &clip.copied_on)? == Some(os)
                && clip
                    .source_app_id
                    .as_ref()
                    .is_some_and(|id| excluded.contains(&id.to_lowercase()))
            {
                continue;
            }
            self.apply_clip(
                &tx,
                clip,
                self_id,
                now,
                &mut applied,
                &mut written_blobs,
                &mut thumbnails,
            )?;
        }
        for board in changes.pinboards {
            if self.apply_pinboard(&tx, board, now)? {
                applied.pinboards_changed = true;
            }
        }
        for item in changes.pinboard_items {
            if self.apply_item(&tx, item, now)? {
                applied.pinboards_changed = true;
            }
        }
        for &id in applied.added.iter().chain(&applied.updated) {
            self.settle_clip(&tx, id, &mut removed)?;
        }
        tx.prepare_cached(
            "UPDATE sync_devices SET received_seq = max(received_seq, ?2), last_seen_at = ?3
             WHERE device_id = ?1",
        )?
        .execute(params![from, changes.max_seq, now])?;
        tx.commit()?;

        // Blobs written for clips that were then skipped, or replaced by newer formats.
        remove_unreferenced_blobs(&conn, &self.blobs, written_blobs)?;
        let removed_ids = self.remove_files(conn, removed)?;
        applied.added.retain(|id| !removed_ids.contains(id));
        applied.updated.retain(|id| !removed_ids.contains(id));
        applied.removed = removed_ids;
        for (content_hash, formats) in thumbnails {
            let sources = formats.into_iter().filter_map(|f: NewFormat| {
                Some((
                    super::thumbs::SourceFormat::from_format_name(&f.format)?,
                    super::thumbs::Source::Bytes(f.data),
                ))
            });
            self.queue_thumbnail(&content_hash, sources);
        }
        if !applied.is_empty() {
            self.changed();
        }
        Ok(applied)
    }

    fn apply_device(&self, tx: &Connection, device: SyncDevice, now: i64) -> StorageResult<bool> {
        let Ok(os) = device.os.parse::<Platform>() else {
            return Ok(false);
        };
        let Ok(name) = check_device_name(&device.name) else {
            return Ok(false);
        };
        if !check_id(&device.device_id) || device.public_key.len() != 32 {
            return Ok(false);
        }
        let hlc = self.clock.observe(device.updated_hlc, now);
        let current: Option<(Vec<u8>, i64, String, String, bool)> = tx
            .prepare_cached(
                "SELECT public_key, updated_hlc, name, os, removed FROM sync_devices
                 WHERE device_id = ?1",
            )?
            .query_row([&device.device_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .optional()?;
        match current {
            None => {
                let taken: bool = tx
                    .prepare_cached(
                        "SELECT EXISTS (SELECT 1 FROM sync_devices WHERE public_key = ?1)",
                    )?
                    .query_row([&device.public_key], |row| row.get(0))?;
                if taken {
                    return Ok(false);
                }
            }
            Some((key, current_hlc, current_name, current_os, removed)) => {
                if key != device.public_key {
                    return Ok(false);
                }
                let theirs = (hlc, device.removed, &name, os.as_str());
                let ours = (current_hlc, removed, &current_name, current_os.as_str());
                if theirs <= ours {
                    return Ok(false);
                }
            }
        }
        upsert_device(
            tx,
            &device.device_id,
            &device.public_key,
            &name,
            os,
            device.removed,
            hlc,
        )?;
        Ok(true)
    }

    fn apply_tombstone(
        &self,
        tx: &Connection,
        tombstone: SyncTombstone,
        now: i64,
        removed: &mut Removed,
        applied: &mut Applied,
    ) -> StorageResult<()> {
        let entity = match tombstone.entity.as_str() {
            Tombstone::CLIP => Tombstone::CLIP,
            Tombstone::PINBOARD => Tombstone::PINBOARD,
            Tombstone::PINBOARD_ITEM => Tombstone::PINBOARD_ITEM,
            _ => return Ok(()),
        };
        if tombstone.key.is_empty() || tombstone.key.len() > MAX_KEY_BYTES {
            return Ok(());
        }
        let hlc = self.clock.observe(tombstone.hlc, now);
        if !put_tombstone(tx, entity, &tombstone.key, hlc)? {
            return Ok(());
        }
        match entity {
            Tombstone::CLIP => {
                let id: Option<i64> = tx
                    .prepare_cached("SELECT id FROM clips WHERE content_hash = ?1")?
                    .query_row([&tombstone.key], |row| row.get(0))
                    .optional()?;
                if let Some(id) = id {
                    self.settle_clip(tx, id, removed)?;
                }
            }
            Tombstone::PINBOARD => {
                // Its items from before the delete go, even where the pinboard itself stays
                // because it was changed again since.
                let items = tx
                    .prepare_cached(
                        "DELETE FROM pinboard_items
                         WHERE pinboard_id = (SELECT id FROM pinboards WHERE uuid = ?1) AND hlc <= ?2",
                    )?
                    .execute(params![tombstone.key, hlc])?;
                applied.pinboards_changed |= items > 0;
                let deleted = tx
                    .prepare_cached("DELETE FROM pinboards WHERE uuid = ?1 AND updated_hlc <= ?2")?
                    .execute(params![tombstone.key, hlc])?;
                applied.pinboards_changed |= deleted > 0;
            }
            _ => {
                let Some((board, clip)) = tombstone.key.split_once('/') else {
                    return Ok(());
                };
                let deleted = tx
                    .prepare_cached(
                        "DELETE FROM pinboard_items
                         WHERE pinboard_id = (SELECT id FROM pinboards WHERE uuid = ?1)
                           AND clip_id = (SELECT id FROM clips WHERE content_hash = ?2)
                           AND hlc <= ?3",
                    )?
                    .execute(params![board, clip, hlc])?;
                applied.pinboards_changed |= deleted > 0;
            }
        }
        Ok(())
    }

    fn apply_clear_range(
        &self,
        tx: &Connection,
        range: SyncClearRange,
        now: i64,
        removed: &mut Removed,
    ) -> StorageResult<()> {
        if range.from_ms.is_some_and(|from| from > range.to_ms) {
            return Ok(());
        }
        let hlc = self.clock.observe(range.hlc, now);
        // Not a UNIQUE conflict: NULLs never conflict, and `from_ms` is often NULL.
        let known: bool = tx
            .prepare_cached(
                "SELECT EXISTS (SELECT 1 FROM clear_ranges
                  WHERE hlc = ?1 AND from_ms IS ?2 AND to_ms = ?3 AND include_kept = ?4)",
            )?
            .query_row(
                params![hlc, range.from_ms, range.to_ms, range.include_kept],
                |row| row.get(0),
            )?;
        if known {
            return Ok(());
        }
        tx.prepare_cached(
            "INSERT INTO clear_ranges (from_ms, to_ms, include_kept, hlc, sync_seq)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?
        .execute(params![
            range.from_ms,
            range.to_ms,
            range.include_kept,
            hlc,
            next_seq(tx)?
        ])?;
        self.settle_range(tx, hlc, range.from_ms, range.to_ms, removed)
    }

    /// Settles every clip a cleared range covers: those last used in its window before it.
    pub(super) fn settle_range(
        &self,
        tx: &Connection,
        hlc: i64,
        from_ms: Option<i64>,
        to_ms: i64,
        removed: &mut Removed,
    ) -> StorageResult<()> {
        let ids: Vec<i64> = tx
            .prepare_cached(
                "SELECT id FROM clips
                 WHERE max(used_hlc, pinned_hlc) < ?1 AND (?2 IS NULL OR last_used_at >= ?2)
                   AND last_used_at <= ?3",
            )?
            .query_map(params![hlc, from_ms, to_ms], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        for id in ids {
            self.settle_clip(tx, id, removed)?;
        }
        Ok(())
    }

    /// Applies the tombstones and cleared ranges to one clip that arrived, changed, or that one
    /// of them covers. The result depends only on the clip's merged values, so every computer
    /// reaches the same one:
    ///
    /// - A tombstone at least as new as the clip's last use, pin or unpin deletes it.
    /// - A cleared range newer than those, whose window holds its last use, deletes it too,
    ///   and leaves a tombstone, so a computer with an older copy deletes it as well.
    /// - Unless every such range keeps kept clips and this one is pinned or on a pinboard. Then
    ///   it's stamped with a new HLC instead, so the ranges no longer cover it anywhere.
    pub(super) fn settle_clip(
        &self,
        tx: &Connection,
        id: i64,
        removed: &mut Removed,
    ) -> StorageResult<()> {
        let row: Option<(String, i64, i64, bool)> = tx
            .prepare_cached(&format!(
                "SELECT c.content_hash, max(c.used_hlc, c.pinned_hlc), c.last_used_at, {KEPT}
                 FROM clips c WHERE c.id = ?1"
            ))?
            .query_row([id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .optional()?;
        let Some((hash, latest, last_used_at, kept)) = row else {
            return Ok(());
        };
        if tombstone_hlc(tx, Tombstone::CLIP, &hash)?.is_some_and(|t| latest <= t) {
            removed.extend(delete_in(tx, [id])?);
            return Ok(());
        }
        let ranges: Vec<(i64, bool)> = tx
            .prepare_cached(
                "SELECT hlc, include_kept FROM clear_ranges
                 WHERE hlc > ?1 AND (from_ms IS NULL OR ?2 >= from_ms) AND ?2 <= to_ms",
            )?
            .query_map(params![latest, last_used_at], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        let Some(newest) = ranges.iter().map(|(hlc, _)| *hlc).max() else {
            return Ok(());
        };
        if kept && ranges.iter().all(|(_, include_kept)| !include_kept) {
            tx.prepare_cached("UPDATE clips SET pinned_hlc = ?2, sync_seq = ?3 WHERE id = ?1")?
                .execute(params![id, self.clock.tick(now_ms()), next_seq(tx)?])?;
        } else {
            removed.extend(delete_in(tx, [id])?);
            put_tombstone(tx, Tombstone::CLIP, &hash, newest)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_clip(
        &self,
        tx: &Connection,
        clip: Checked,
        self_id: &str,
        now: i64,
        applied: &mut Applied,
        written_blobs: &mut Vec<String>,
        thumbnails: &mut Vec<(String, Vec<NewFormat>)>,
    ) -> StorageResult<()> {
        let used_hlc = self.clock.observe(clip.used_hlc, now);
        let pinned_hlc = self.clock.observe(clip.pinned_hlc, now);
        let copied_on = (clip.copied_on != self_id).then(|| clip.copied_on.clone());
        let theirs: UseKey = (
            used_hlc,
            clip.last_used_at,
            clip.source_app_id.clone(),
            clip.source_app_name.clone(),
            Some(clip.copied_on.clone()),
        );

        type Local = (i64, UseKey, i64, bool, i64);
        let local: Option<Local> = tx
            .prepare_cached(
                "SELECT id, used_hlc, last_used_at, source_app_id, source_app_name, copied_on,
                        created_at, pinned, pinned_hlc
                 FROM clips WHERE content_hash = ?1",
            )?
            .query_row([&clip.content_hash], |row| {
                Ok((
                    row.get(0)?,
                    (
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        Some(
                            row.get::<_, Option<String>>(5)?
                                .unwrap_or_else(|| self_id.to_owned()),
                        ),
                    ),
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            })
            .optional()?;

        let Some((id, ours, created_at, pinned, our_pinned_hlc)) = local else {
            let tombstone: Option<i64> = tx
                .prepare_cached("SELECT hlc FROM tombstones WHERE entity = 'clip' AND key = ?1")?
                .query_row([&clip.content_hash], |row| row.get(0))
                .optional()?;
            // Cleared ranges are checked once its pinboard items are in, by settle_clip.
            if tombstone.is_some_and(|t| used_hlc.max(pinned_hlc) <= t) {
                return Ok(());
            }
            let id = self.insert_clip(tx, &clip, used_hlc, pinned_hlc, copied_on, written_blobs)?;
            applied.added.push(id);
            if clip.kind == ClipKind::Image {
                thumbnails.push((clip.content_hash, clip.formats));
            }
            return Ok(());
        };

        let mut changed = false;
        if theirs > ours {
            tx.prepare_cached(
                "UPDATE clips SET last_used_at = ?2, source_app_name = ?3, source_app_id = ?4,
                                  copied_on = ?5, used_hlc = ?6
                 WHERE id = ?1",
            )?
            .execute(params![
                id,
                clip.last_used_at,
                clip.source_app_name,
                clip.source_app_id,
                copied_on,
                used_hlc
            ])?;
            // Rich text comes with the formatting of its latest use.
            if clip.kind == ClipKind::RichText {
                let replaced: Vec<String> = tx
                    .prepare_cached(
                        "SELECT blob_hash FROM clip_formats WHERE clip_id = ?1 AND blob_hash IS NOT NULL",
                    )?
                    .query_map([id], |row| row.get(0))?
                    .collect::<Result<_, _>>()?;
                tx.execute("DELETE FROM clip_formats WHERE clip_id = ?1", [id])?;
                written_blobs.extend(replaced);
                let size = self.insert_formats(tx, id, &clip, written_blobs)?;
                tx.prepare_cached("UPDATE clips SET size_bytes = ?2 WHERE id = ?1")?
                    .execute(params![id, size])?;
            }
            changed = true;
        }
        if clip.created_at < created_at {
            tx.prepare_cached("UPDATE clips SET created_at = ?2 WHERE id = ?1")?
                .execute(params![id, clip.created_at])?;
            changed = true;
        }
        if (pinned_hlc, clip.pinned) > (our_pinned_hlc, pinned) {
            tx.prepare_cached("UPDATE clips SET pinned = ?2, pinned_hlc = ?3 WHERE id = ?1")?
                .execute(params![id, clip.pinned, pinned_hlc])?;
            changed = true;
        }
        if changed {
            tx.prepare_cached("UPDATE clips SET sync_seq = ?2 WHERE id = ?1")?
                .execute(params![id, next_seq(tx)?])?;
            applied.updated.push(id);
        }
        Ok(())
    }

    /// Inserts a received clip that isn't here yet, and returns its id.
    fn insert_clip(
        &self,
        tx: &Connection,
        clip: &Checked,
        used_hlc: i64,
        pinned_hlc: i64,
        copied_on: Option<String>,
        written_blobs: &mut Vec<String>,
    ) -> StorageResult<i64> {
        let id: i64 = tx
            .prepare_cached(
                "INSERT INTO clips (uuid, kind, plain_text, content_hash, size_bytes,
                                    source_app_name, source_app_id, created_at, last_used_at,
                                    use_count, pinned, metadata, used_hlc, pinned_hlc, sync_seq,
                                    copied_on)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11, ?12, ?13, ?14)
                 RETURNING id",
            )?
            .query_row(
                params![
                    uuid::Uuid::now_v7().to_string(),
                    clip.kind.as_str(),
                    clip.plain_text,
                    clip.content_hash,
                    clip.source_app_name,
                    clip.source_app_id,
                    clip.created_at,
                    clip.last_used_at,
                    clip.pinned,
                    clip.metadata,
                    used_hlc,
                    pinned_hlc,
                    next_seq(tx)?,
                    copied_on,
                ],
                |row| row.get(0),
            )?;
        let size = self.insert_formats(tx, id, clip, written_blobs)?;
        tx.prepare_cached("UPDATE clips SET size_bytes = ?2 WHERE id = ?1")?
            .execute(params![id, size])?;
        Ok(id)
    }

    /// Stores a received clip's formats, as [`upsert_clip`](Store::upsert_clip) would, and
    /// returns the clip's size.
    fn insert_formats(
        &self,
        tx: &Connection,
        clip_id: i64,
        clip: &Checked,
        written_blobs: &mut Vec<String>,
    ) -> StorageResult<i64> {
        let platform = Platform::MacOs;
        let plain_text_format = platform.plain_text_format();
        let full_text = clip
            .formats
            .iter()
            .filter(|f| f.format == plain_text_format)
            .count();
        let mut size = clip.plain_text.as_ref().map_or(0, String::len);
        let mut insert = tx.prepare_cached(
            "INSERT OR IGNORE INTO clip_formats
               (clip_id, item_index, platform, format, data, blob_hash, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for f in &clip.formats {
            // The plain text the sender put back, when it's the clip's text: not stored, as
            // when it's copied here.
            let duplicate = full_text == 1
                && f.format == plain_text_format
                && clip.plain_text.as_deref().map(str::as_bytes) == Some(&f.data[..]);
            if duplicate {
                continue;
            }
            let blob_hash = if f.data.len() > MAX_INLINE_BYTES {
                let hash = self.blobs.put(&f.data)?;
                written_blobs.push(hash.clone());
                Some(hash)
            } else {
                None
            };
            let data = blob_hash.is_none().then_some(&f.data[..]);
            insert.execute(params![
                clip_id,
                f.item_index,
                platform.as_str(),
                f.format,
                data,
                blob_hash,
                f.data.len() as i64,
            ])?;
            size += f.data.len();
        }
        Ok(size as i64)
    }

    fn apply_pinboard(
        &self,
        tx: &Connection,
        board: SyncPinboard,
        now: i64,
    ) -> StorageResult<bool> {
        if !check_id(&board.uuid) {
            return Ok(false);
        }
        let (Ok(name), Ok(color)) = (check_name(&board.name), check_color(board.color.as_deref()))
        else {
            return Ok(false);
        };
        let position = board.position.clamp(0, i64::from(u32::MAX));
        let hlc = self.clock.observe(board.updated_hlc, now);
        if tombstone_hlc(tx, Tombstone::PINBOARD, &board.uuid)?.is_some_and(|t| t >= hlc) {
            return Ok(false);
        }
        let current: Option<(i64, i64, String, Option<String>, i64)> = tx
            .prepare_cached(
                "SELECT id, updated_hlc, name, color, position FROM pinboards WHERE uuid = ?1",
            )?
            .query_row([&board.uuid], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .optional()?;
        match current {
            None => {
                tx.prepare_cached(
                    "INSERT INTO pinboards (uuid, name, color, position, created_at, updated_at,
                                            updated_hlc, sync_seq)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )?
                .execute(params![
                    board.uuid,
                    name,
                    color,
                    position,
                    board.created_at,
                    now,
                    hlc,
                    next_seq(tx)?
                ])?;
            }
            Some((id, current_hlc, current_name, current_color, current_position)) => {
                let theirs = (hlc, &name, &color, position);
                let ours = (current_hlc, &current_name, &current_color, current_position);
                if theirs <= ours {
                    return Ok(false);
                }
                tx.prepare_cached(
                    "UPDATE pinboards SET name = ?2, color = ?3, position = ?4, updated_at = ?5,
                                          updated_hlc = ?6, sync_seq = ?7
                     WHERE id = ?1",
                )?
                .execute(params![
                    id,
                    name,
                    color,
                    position,
                    now,
                    hlc,
                    next_seq(tx)?
                ])?;
            }
        }
        Ok(true)
    }

    fn apply_item(&self, tx: &Connection, item: SyncPinboardItem, now: i64) -> StorageResult<bool> {
        let hlc = self.clock.observe(item.hlc, now);
        let key = Tombstone::item_key(&item.pinboard, &item.clip);
        if key.len() > MAX_KEY_BYTES {
            return Ok(false);
        }
        // Taking it off, or deleting its pinboard, removes it, if that came later. Deleting
        // its clip removes it with the clip.
        for (entity, key) in [
            (Tombstone::PINBOARD_ITEM, key.as_str()),
            (Tombstone::PINBOARD, item.pinboard.as_str()),
        ] {
            if tombstone_hlc(tx, entity, key)?.is_some_and(|t| t >= hlc) {
                return Ok(false);
            }
        }
        let ids: Option<(Option<i64>, Option<i64>)> = tx
            .prepare_cached(
                "SELECT (SELECT id FROM pinboards WHERE uuid = ?1),
                        (SELECT id FROM clips WHERE content_hash = ?2)",
            )?
            .query_row(params![item.pinboard, item.clip], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;
        let Some((Some(pinboard_id), Some(clip_id))) = ids else {
            return Ok(false);
        };
        let current: Option<(i64, i64)> = tx
            .prepare_cached(
                "SELECT hlc, position FROM pinboard_items WHERE pinboard_id = ?1 AND clip_id = ?2",
            )?
            .query_row(params![pinboard_id, clip_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;
        if current.is_some_and(|ours| (hlc, item.position) <= ours) {
            return Ok(false);
        }
        tx.prepare_cached(
            "INSERT INTO pinboard_items (pinboard_id, clip_id, position, added_at, hlc, sync_seq)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (pinboard_id, clip_id) DO UPDATE SET
               position = excluded.position, added_at = excluded.added_at, hlc = excluded.hlc,
               sync_seq = excluded.sync_seq",
        )?
        .execute(params![
            pinboard_id,
            clip_id,
            item.position,
            item.added_at,
            hlc,
            next_seq(tx)?
        ])?;
        Ok(true)
    }
}

/// Checks a received clip: its fields, its size, and that its hash matches its content.
fn check_clip(clip: SyncClip) -> Option<Checked> {
    let kind: ClipKind = clip.kind.parse().ok()?;
    blobs::check_hash(&clip.content_hash).ok()?;
    let total: u64 = clip.formats.iter().map(|f| f.data.len() as u64).sum();
    let formats_ok = clip.formats.len() <= MAX_FORMATS
        && total <= MAX_RECEIVED_CLIP_BYTES
        && clip.formats.iter().all(|f| {
            !f.format.is_empty()
                && f.format.len() <= MAX_FORMAT_NAME_BYTES
                && f.item_index <= MAX_ITEM_INDEX
        });
    let metadata_ok = clip.metadata.as_ref().is_none_or(|m| {
        m.len() <= MAX_METADATA_BYTES
            && serde_json::from_str::<serde_json::Value>(m).is_ok_and(|v| v.is_object())
    });
    let short = |s: &Option<String>| s.as_ref().is_none_or(|s| s.len() <= MAX_APP_BYTES);
    let fields_ok = metadata_ok
        && short(&clip.source_app_name)
        && short(&clip.source_app_id)
        && check_id(&clip.copied_on)
        && clip.created_at >= 0
        && clip.last_used_at >= 0;
    if !formats_ok || !fields_ok {
        return None;
    }
    let formats: Vec<NewFormat> = clip
        .formats
        .into_iter()
        .map(|f| NewFormat {
            item_index: f.item_index,
            format: f.format,
            data: f.data,
        })
        .collect();
    let plain_text = crate::classifier::verify(kind, &clip.content_hash, &formats)?;
    Some(Checked {
        kind,
        content_hash: clip.content_hash,
        plain_text,
        formats,
        metadata: clip.metadata,
        created_at: clip.created_at,
        last_used_at: clip.last_used_at,
        used_hlc: clip.used_hlc,
        source_app_name: clip.source_app_name,
        source_app_id: clip.source_app_id,
        copied_on: clip.copied_on,
        pinned: clip.pinned,
        pinned_hlc: clip.pinned_hlc,
    })
}

fn read_pinboard(conn: &Connection, id: i64) -> StorageResult<Option<SyncPinboard>> {
    Ok(conn
        .prepare_cached(
            "SELECT uuid, name, color, position, created_at, updated_hlc FROM pinboards
             WHERE id = ?1",
        )?
        .query_row([id], |row| {
            Ok(SyncPinboard {
                uuid: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                position: row.get(3)?,
                created_at: row.get(4)?,
                updated_hlc: row.get(5)?,
            })
        })
        .optional()?)
}

fn read_item(
    conn: &Connection,
    pinboard_id: i64,
    clip_id: i64,
) -> StorageResult<Option<SyncPinboardItem>> {
    Ok(conn
        .prepare_cached(
            "SELECT b.uuid, c.content_hash, i.position, i.added_at, i.hlc
             FROM pinboard_items i
             JOIN pinboards b ON b.id = i.pinboard_id
             JOIN clips c ON c.id = i.clip_id
             WHERE i.pinboard_id = ?1 AND i.clip_id = ?2",
        )?
        .query_row(params![pinboard_id, clip_id], |row| {
            Ok(SyncPinboardItem {
                pinboard: row.get(0)?,
                clip: row.get(1)?,
                position: row.get(2)?,
                added_at: row.get(3)?,
                hlc: row.get(4)?,
            })
        })
        .optional()?)
}

fn upsert_device(
    tx: &Connection,
    device_id: &str,
    public_key: &[u8],
    name: &str,
    os: Platform,
    removed: bool,
    hlc: i64,
) -> rusqlite::Result<()> {
    tx.prepare_cached(
        "INSERT INTO sync_devices (device_id, public_key, name, os, removed, updated_hlc, sync_seq)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (device_id) DO UPDATE SET
           public_key = excluded.public_key, name = excluded.name, os = excluded.os,
           removed = excluded.removed, updated_hlc = excluded.updated_hlc,
           sync_seq = excluded.sync_seq",
    )?
    .execute(params![
        device_id,
        public_key,
        name,
        os.as_str(),
        removed,
        hlc,
        next_seq(tx)?
    ])?;
    Ok(())
}

fn tombstone_hlc(tx: &Connection, entity: &str, key: &str) -> rusqlite::Result<Option<i64>> {
    tx.prepare_cached("SELECT hlc FROM tombstones WHERE entity = ?1 AND key = ?2")?
        .query_row(params![entity, key], |row| row.get(0))
        .optional()
}

/// The OS of a computer in the group, if it's known.
fn device_os(tx: &Connection, device_id: &str) -> StorageResult<Option<Platform>> {
    let os: Option<String> = tx
        .prepare_cached("SELECT os FROM sync_devices WHERE device_id = ?1")?
        .query_row([device_id], |row| row.get(0))
        .optional()?;
    Ok(os.and_then(|os| os.parse().ok()))
}

/// The platform this build runs on, whose excluded apps apply.
fn local_platform() -> Platform {
    match kopi_platform::current_os() {
        kopi_platform::Os::Windows => Platform::Windows,
        _ => Platform::MacOs,
    }
}

/// A device ID or pinboard uuid: 1 to 64 characters of letters, digits and dashes.
fn check_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_BYTES
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// A trimmed computer name of 1 to [`MAX_DEVICE_NAME_CHARS`] characters, without control
/// characters.
pub fn check_device_name(name: &str) -> StorageResult<String> {
    let name: String = name.trim().chars().filter(|c| !c.is_control()).collect();
    match name.chars().count() {
        0 => Err(StorageError::Invalid("a computer needs a name".into())),
        n if n > MAX_DEVICE_NAME_CHARS => Err(StorageError::Invalid(format!(
            "a computer's name can be at most {MAX_DEVICE_NAME_CHARS} characters"
        ))),
        _ => Ok(name),
    }
}

/// Roughly how many bytes a clip takes on the wire.
fn clip_bytes(clip: &SyncClip) -> usize {
    clip.formats.iter().map(|f| f.data.len()).sum::<usize>() + 256
}

/// Adds a PNG to an image held only as TIFF, since Windows can't paste TIFF. Skipped if the
/// TIFF doesn't decode or the clip would pass the capture limit.
fn add_png(formats: &mut Vec<SyncFormat>) {
    if formats.iter().any(|f| f.format == uti::PNG) {
        return;
    }
    let Some(tiff) = formats.iter().find(|f| f.format == uti::TIFF) else {
        return;
    };
    let Ok(image) = image::load_from_memory_with_format(&tiff.data, image::ImageFormat::Tiff)
    else {
        return;
    };
    let mut png = Cursor::new(Vec::new());
    if image.write_to(&mut png, image::ImageFormat::Png).is_err() {
        return;
    }
    let png = png.into_inner();
    let total: usize = formats.iter().map(|f| f.data.len()).sum::<usize>() + png.len();
    if total as u64 > DEFAULT_MAX_CLIP_BYTES {
        return;
    }
    let item_index = tiff.item_index;
    formats.push(SyncFormat {
        item_index,
        format: uti::PNG.to_owned(),
        data: png,
    });
}
