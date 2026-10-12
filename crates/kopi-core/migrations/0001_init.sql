-- The first schema. See docs/DATA_MODEL.md for what each table and column is for.

CREATE TABLE clips (
  id               INTEGER PRIMARY KEY,
  uuid             TEXT    NOT NULL UNIQUE,
  kind             TEXT    NOT NULL CHECK (kind IN
                     ('text','rich_text','link','image','file','color','code')),
  plain_text       TEXT,
  content_hash     TEXT    NOT NULL UNIQUE,
  size_bytes       INTEGER NOT NULL,
  source_app_name  TEXT,
  source_app_id    TEXT,
  created_at       INTEGER NOT NULL,
  last_used_at     INTEGER NOT NULL,
  use_count        INTEGER NOT NULL DEFAULT 1,
  pinned           INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0,1)),
  metadata         TEXT
);

CREATE TABLE clip_formats (
  id          INTEGER PRIMARY KEY,
  clip_id     INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
  item_index  INTEGER NOT NULL DEFAULT 0,
  platform    TEXT    NOT NULL CHECK (platform IN ('macos','windows')),
  format      TEXT    NOT NULL,
  data        BLOB,
  blob_hash   TEXT,
  size_bytes  INTEGER NOT NULL,
  CHECK ((data IS NULL) <> (blob_hash IS NULL)),
  UNIQUE (clip_id, item_index, format)
);

CREATE TABLE pinboards (
  id          INTEGER PRIMARY KEY,
  uuid        TEXT    NOT NULL UNIQUE,
  name        TEXT    NOT NULL,
  color       TEXT,
  position    INTEGER NOT NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

CREATE TABLE pinboard_items (
  pinboard_id INTEGER NOT NULL REFERENCES pinboards(id) ON DELETE CASCADE,
  clip_id     INTEGER NOT NULL REFERENCES clips(id)     ON DELETE CASCADE,
  position    INTEGER NOT NULL,
  added_at    INTEGER NOT NULL,
  PRIMARY KEY (pinboard_id, clip_id)
) WITHOUT ROWID;

CREATE TABLE settings (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL,
  updated_at  INTEGER NOT NULL
) WITHOUT ROWID;

CREATE TABLE excluded_apps (
  id          INTEGER PRIMARY KEY,
  platform    TEXT    NOT NULL CHECK (platform IN ('macos','windows')),
  app_id      TEXT    NOT NULL,
  app_name    TEXT    NOT NULL,
  origin      TEXT    NOT NULL CHECK (origin IN ('default','user')),
  created_at  INTEGER NOT NULL,
  UNIQUE (platform, app_id)
);

CREATE VIRTUAL TABLE clips_fts USING fts5(
  plain_text,
  source_app_name,
  content = 'clips',
  content_rowid = 'id',
  tokenize = 'trigram remove_diacritics 1'
);
-- Purge deleted terms from the index too, not just mark them deleted.
INSERT INTO clips_fts(clips_fts, rank) VALUES ('secure-delete', 1);

CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
  INSERT INTO clips_fts(rowid, plain_text, source_app_name)
  VALUES (new.id, new.plain_text, new.source_app_name);
END;
CREATE TRIGGER clips_ad AFTER DELETE ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, plain_text, source_app_name)
  VALUES ('delete', old.id, old.plain_text, old.source_app_name);
END;
-- Only re-index when an indexed value actually changes, not on every last_used_at bump
-- or repeat copy from the same app.
CREATE TRIGGER clips_au AFTER UPDATE OF plain_text, source_app_name ON clips
WHEN old.plain_text IS NOT new.plain_text OR old.source_app_name IS NOT new.source_app_name BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, plain_text, source_app_name)
  VALUES ('delete', old.id, old.plain_text, old.source_app_name);
  INSERT INTO clips_fts(rowid, plain_text, source_app_name)
  VALUES (new.id, new.plain_text, new.source_app_name);
END;

CREATE INDEX clips_by_recent  ON clips (last_used_at DESC);
CREATE INDEX clips_by_kind    ON clips (kind, last_used_at DESC);
CREATE INDEX clips_by_app     ON clips (source_app_id, last_used_at DESC);
CREATE INDEX clips_pinned     ON clips (last_used_at DESC) WHERE pinned = 1;
CREATE INDEX clip_formats_blob ON clip_formats (blob_hash) WHERE blob_hash IS NOT NULL;
CREATE INDEX pinboard_items_by_clip ON pinboard_items (clip_id);
