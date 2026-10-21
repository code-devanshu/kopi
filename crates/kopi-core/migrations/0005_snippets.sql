-- Snippets: text the user writes once and pastes from the panel. See docs/SNIPPETS_DESIGN.md.
--
-- The body is stored as typed, placeholders and all. The panel fills them in when it pastes.
-- Retention and Clear history never touch this table.

CREATE TABLE snippets (
  id            INTEGER PRIMARY KEY,
  uuid          TEXT    NOT NULL UNIQUE,  -- UUIDv7, its identity across computers
  name          TEXT    NOT NULL,         -- 1–64 characters
  keyword       TEXT,                     -- 1–32 characters, no whitespace; not unique
  body          TEXT    NOT NULL,         -- up to 64 KiB
  position      INTEGER NOT NULL,         -- order in the tab, lowest first
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  last_used_at  INTEGER,                  -- this computer only
  use_count     INTEGER NOT NULL DEFAULT 0, -- this computer only
  updated_hlc   INTEGER NOT NULL,
  sync_seq      INTEGER NOT NULL
);

CREATE INDEX snippets_by_sync_seq ON snippets (sync_seq);

-- Tombstones gain 'snippet', keyed by uuid. SQLite can't change a CHECK constraint in place,
-- so the table is rebuilt with its rows.
CREATE TABLE tombstones_new (
  entity    TEXT    NOT NULL CHECK (entity IN ('clip','pinboard','pinboard_item','snippet')),
  -- A content_hash, a pinboard or snippet uuid, or "<pinboard uuid>/<content_hash>".
  key       TEXT    NOT NULL,
  hlc       INTEGER NOT NULL,
  sync_seq  INTEGER NOT NULL,
  PRIMARY KEY (entity, key)
) WITHOUT ROWID;

INSERT INTO tombstones_new (entity, key, hlc, sync_seq)
SELECT entity, key, hlc, sync_seq FROM tombstones;

DROP TABLE tombstones;
ALTER TABLE tombstones_new RENAME TO tombstones;
CREATE INDEX tombstones_by_sync_seq ON tombstones (sync_seq);

-- How far snippets have been received from each computer. Older versions skipped them while
-- moving received_seq on, so it starts at 0 and the first session fetches every snippet.
ALTER TABLE sync_devices ADD COLUMN snippets_seq INTEGER NOT NULL DEFAULT 0;
