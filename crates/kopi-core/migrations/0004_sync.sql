-- Change tracking for sync between computers. See docs/SYNC_DESIGN.md.
--
-- Filled for every change whether sync is on or not, so turning it on later has a starting
-- point. An HLC is Unix milliseconds shifted left 16 bits, plus a counter in the low 16 bits.
-- Existing rows get HLCs from their timestamps, and sync_seqs in the order they were last used.

ALTER TABLE clips ADD COLUMN used_hlc   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clips ADD COLUMN pinned_hlc INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clips ADD COLUMN sync_seq   INTEGER NOT NULL DEFAULT 0;
-- The device ID of the computer the clip was last copied on. NULL is this computer.
ALTER TABLE clips ADD COLUMN copied_on  TEXT;
-- Never synced: a Windows copy marked CanUploadToCloudClipboard = 0.
ALTER TABLE clips ADD COLUMN local_only INTEGER NOT NULL DEFAULT 0 CHECK (local_only IN (0,1));

ALTER TABLE pinboards      ADD COLUMN updated_hlc INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pinboards      ADD COLUMN sync_seq    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pinboard_items ADD COLUMN hlc         INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pinboard_items ADD COLUMN sync_seq    INTEGER NOT NULL DEFAULT 0;

UPDATE clips SET used_hlc = last_used_at << 16,
                 pinned_hlc = CASE WHEN pinned = 1 THEN last_used_at << 16 ELSE 0 END;
UPDATE pinboards SET updated_hlc = updated_at << 16;
UPDATE pinboard_items SET hlc = added_at << 16;

UPDATE clips SET sync_seq = r.n
FROM (SELECT id, row_number() OVER (ORDER BY last_used_at, id) AS n FROM clips) AS r
WHERE r.id = clips.id;
UPDATE pinboards SET sync_seq = r.n + (SELECT count(*) FROM clips)
FROM (SELECT id, row_number() OVER (ORDER BY position, id) AS n FROM pinboards) AS r
WHERE r.id = pinboards.id;
UPDATE pinboard_items SET sync_seq = r.n + (SELECT count(*) FROM clips) + (SELECT count(*) FROM pinboards)
FROM (SELECT pinboard_id, clip_id,
             row_number() OVER (ORDER BY pinboard_id, position, clip_id) AS n
      FROM pinboard_items) AS r
WHERE r.pinboard_id = pinboard_items.pinboard_id AND r.clip_id = pinboard_items.clip_id;

-- The last sync_seq given out. Every change to a synced row takes the next one.
INSERT INTO settings (key, value, updated_at)
VALUES ('sync.seq',
        (SELECT count(*) FROM clips) + (SELECT count(*) FROM pinboards)
          + (SELECT count(*) FROM pinboard_items),
        CAST(unixepoch('subsec') * 1000 AS INTEGER));

-- An explicit delete, so it reaches every computer. Kept for 90 days.
CREATE TABLE tombstones (
  entity    TEXT    NOT NULL CHECK (entity IN ('clip','pinboard','pinboard_item')),
  -- A content_hash, a pinboard uuid, or "<pinboard uuid>/<content_hash>".
  key       TEXT    NOT NULL,
  hlc       INTEGER NOT NULL,
  sync_seq  INTEGER NOT NULL,
  PRIMARY KEY (entity, key)
) WITHOUT ROWID;

-- Clear history: the clips last used in [from_ms, to_ms] before hlc. Kept for 90 days.
CREATE TABLE clear_ranges (
  id            INTEGER PRIMARY KEY,
  from_ms       INTEGER,          -- NULL = from the beginning
  to_ms         INTEGER NOT NULL,
  include_kept  INTEGER NOT NULL CHECK (include_kept IN (0,1)),
  hlc           INTEGER NOT NULL,
  sync_seq      INTEGER NOT NULL
);

-- The sync group: this computer and the ones paired with it. Synced like any other data.
CREATE TABLE sync_devices (
  device_id     TEXT    PRIMARY KEY,
  public_key    BLOB    NOT NULL UNIQUE,
  name          TEXT    NOT NULL,
  os            TEXT    NOT NULL CHECK (os IN ('macos','windows')),
  removed       INTEGER NOT NULL DEFAULT 0 CHECK (removed IN (0,1)),
  updated_hlc   INTEGER NOT NULL,
  sync_seq      INTEGER NOT NULL,
  -- The highest of its sync_seqs applied here.
  received_seq  INTEGER NOT NULL DEFAULT 0,
  last_seen_at  INTEGER
) WITHOUT ROWID;

CREATE INDEX clips_by_sync_seq          ON clips (sync_seq);
CREATE INDEX pinboards_by_sync_seq      ON pinboards (sync_seq);
CREATE INDEX pinboard_items_by_sync_seq ON pinboard_items (sync_seq);
CREATE INDEX tombstones_by_sync_seq     ON tombstones (sync_seq);
CREATE INDEX clear_ranges_by_sync_seq   ON clear_ranges (sync_seq);
CREATE INDEX sync_devices_by_sync_seq   ON sync_devices (sync_seq);
