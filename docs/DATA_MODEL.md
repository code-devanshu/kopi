# Data model

This is the planned SQLite schema for the desktop app, and the rules for search, files on disk, duplicates and retention. For why we chose SQLite and FTS5, see [ADR 0002](decisions/0002-sqlite-fts5-storage.md).

## Storage location

Everything lives in the app's **local** data folder. Clipboard history must never roam between machines through OS profile sync. The bundle identifier is planned to be `in.devanshuverma.kopi`.

| OS | Folder (Tauri `app_local_data_dir`) |
| --- | --- |
| macOS | `~/Library/Application Support/in.devanshuverma.kopi/` |
| Windows | `%LOCALAPPDATA%\in.devanshuverma.kopi\` |

```text
<data dir>/
├── kopi.db                 # SQLite database (plus kopi.db-wal and kopi.db-shm)
├── blobs/                  # Large format data, content-addressed
│   └── 3f/a9/3fa9…e1.bin   # <blake3 hex>, sharded by the first two bytes
└── thumbs/
    └── 3f/3fa9…e1@320.png  # Image thumbnails, longest edge 320 px
```

The folder is created with user-only permissions (`0700` on macOS). Logs never contain clip content.

## Connection settings

```sql
PRAGMA page_size = 8192;        -- set before the first table is created
PRAGMA journal_mode = WAL;      -- readers don't block the single writer
PRAGMA synchronous = NORMAL;    -- safe with WAL; a power cut can lose the last few clips, not corrupt the DB
PRAGMA foreign_keys = ON;
PRAGMA secure_delete = ON;      -- overwrite deleted content in the DB file
```

The schema version is tracked with `PRAGMA user_version`, and migrations are plain SQL files in `crates/kopi-core/migrations/`, applied in order inside a transaction.

## Tables

All timestamps are Unix milliseconds (UTC). `uuid` columns are UUIDv7, giving each row a stable ID for sync later. The integer `id` is kept because FTS5 needs an integer rowid.

### `clips`

One row per distinct piece of content in the history.

```sql
CREATE TABLE clips (
  id               INTEGER PRIMARY KEY,
  uuid             TEXT    NOT NULL UNIQUE,
  kind             TEXT    NOT NULL CHECK (kind IN
                     ('text','rich_text','link','image','file','color','code')),
  plain_text       TEXT,             -- display + search text, max 64 KiB (see below)
  content_hash     TEXT    NOT NULL UNIQUE,  -- BLAKE3 hex, see "Duplicates"
  size_bytes       INTEGER NOT NULL, -- total size of all stored formats
  source_app_name  TEXT,             -- "Safari"
  source_app_id    TEXT,             -- macOS bundle ID "com.apple.Safari"; Windows exe path or AUMID
  created_at       INTEGER NOT NULL, -- first time this content was seen
  last_used_at     INTEGER NOT NULL, -- last copy OR paste; drives the history order
  use_count        INTEGER NOT NULL DEFAULT 1,  -- copies + pastes
  pinned           INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0,1)),
  metadata         TEXT              -- JSON: {w,h} for images, {rgba} for colors,
                                     -- {host} for links, {count,names} for files, {lang} for code
);
```

`plain_text` per kind:

| Kind | `plain_text` holds |
| --- | --- |
| `text`, `code` | The text |
| `rich_text` | The plain-text representation (formatting lives in `clip_formats`) |
| `link` | The URL |
| `color` | Normalised hex, such as `#1E90FFFF` |
| `file` | File names, one per line (full paths go in `clip_formats`) |
| `image` | `NULL` for now (OCR is out of scope) |

Text longer than 64 KiB is truncated in `plain_text`, and the full text is kept in `clip_formats`. This keeps the list and the search index fast, while paste still uses the full content.

### `clip_formats`

The raw representations that were on the clipboard, so paste can put back exactly what was copied (rich text, HTML, images, file URLs).

```sql
CREATE TABLE clip_formats (
  id          INTEGER PRIMARY KEY,
  clip_id     INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
  item_index  INTEGER NOT NULL DEFAULT 0,  -- pasteboard item (macOS can hold several, e.g. 3 files)
  platform    TEXT    NOT NULL CHECK (platform IN ('macos','windows')),
  format      TEXT    NOT NULL,  -- macOS UTI ("public.rtf") or Windows format name ("HTML Format", "CF_UNICODETEXT")
  data        BLOB,              -- inline when ≤ 64 KiB
  blob_hash   TEXT,              -- file in blobs/ when > 64 KiB
  size_bytes  INTEGER NOT NULL,
  CHECK ((data IS NULL) <> (blob_hash IS NULL)),
  UNIQUE (clip_id, item_index, format)
);
```

- Only the formats in the allow-list below are stored. Private app formats (such as Microsoft Office's internal ones) are skipped in the alpha to save space.
- When the plain-text format would be an exact copy of an untruncated `clips.plain_text`, no row is written for it.
- Blob files are content-addressed, so two clips with the same image share one file. A file is deleted only when no row references it.

Allowed formats in the alpha:

| Purpose | macOS (UTI) | Windows |
| --- | --- | --- |
| Plain text | `public.utf8-plain-text` | `CF_UNICODETEXT` |
| Rich text | `public.rtf` | `Rich Text Format` |
| HTML | `public.html` | `HTML Format` |
| Image | `public.png`, `public.tiff` | `PNG`, `CF_DIBV5` |
| Files | `public.file-url` (one item per file) | `CF_HDROP` |
| Color | `com.apple.cocoa.pasteboard.color` | n/a (detected from text) |
| URL | `public.url` | `UniformResourceLocatorW` |

### `pinboards` and `pinboard_items`

Named collections that the user keeps. A clip can be on several pinboards. Being on a pinboard protects a clip from retention, in the same way as `pinned`.

```sql
CREATE TABLE pinboards (
  id          INTEGER PRIMARY KEY,
  uuid        TEXT    NOT NULL UNIQUE,
  name        TEXT    NOT NULL,
  color       TEXT,                 -- small visual marker, e.g. "#F59E0B"
  position    INTEGER NOT NULL,     -- order in the sidebar
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
```

`pinned` on `clips` is the quick "keep this" toggle, and pinned clips show in a Pinned section at the top. Pinboards are for organising. The two are independent.

### `settings`

```sql
CREATE TABLE settings (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL,        -- JSON
  updated_at  INTEGER NOT NULL
) WITHOUT ROWID;
```

Keys are defined and validated in Rust, with defaults applied when a key is missing. Examples include `hotkey.open_panel`, `capture.paused`, `capture.kinds`, `history.max_items`, `history.max_age_days`, `history.max_total_bytes`, `limits.max_clip_bytes`, `paste.default_plain_text`, `app.launch_at_login` and `ui.theme`.

### `excluded_apps`

Apps that Kopi never captures from.

```sql
CREATE TABLE excluded_apps (
  id          INTEGER PRIMARY KEY,
  platform    TEXT    NOT NULL CHECK (platform IN ('macos','windows')),
  app_id      TEXT    NOT NULL,     -- bundle ID / exe path
  app_name    TEXT    NOT NULL,
  origin      TEXT    NOT NULL CHECK (origin IN ('default','user')),
  created_at  INTEGER NOT NULL,
  UNIQUE (platform, app_id)
);
```

These rows are seeded on first run with common password managers and Keychain Access. The user can remove them, and removed defaults aren't re-added. See [PRIVACY_DESIGN.md](PRIVACY_DESIGN.md).

## Full-text search (FTS5)

```sql
CREATE VIRTUAL TABLE clips_fts USING fts5(
  plain_text,
  source_app_name,
  content = 'clips',
  content_rowid = 'id',
  tokenize = 'trigram remove_diacritics 1'
);
-- Purge deleted terms from the index too, not just mark them deleted (SQLite 3.42+):
INSERT INTO clips_fts(clips_fts, rank) VALUES ('secure-delete', 1);

CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
  INSERT INTO clips_fts(rowid, plain_text, source_app_name)
  VALUES (new.id, new.plain_text, new.source_app_name);
END;
CREATE TRIGGER clips_ad AFTER DELETE ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, plain_text, source_app_name)
  VALUES ('delete', old.id, old.plain_text, old.source_app_name);
END;
-- Only re-index when indexed columns change, not on every last_used_at bump:
CREATE TRIGGER clips_au AFTER UPDATE OF plain_text, source_app_name ON clips BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, plain_text, source_app_name)
  VALUES ('delete', old.id, old.plain_text, old.source_app_name);
  INSERT INTO clips_fts(rowid, plain_text, source_app_name)
  VALUES (new.id, new.plain_text, new.source_app_name);
END;
```

Why these choices:

- **External content** (`content='clips'`): the text is stored once, and the FTS table holds only the index.
- **Trigram tokenizer**: clipboard search is mostly about fragments, such as part of a URL, a variable name or the middle of a token. Trigram matches any substring of 3 or more characters. A word tokenizer can't find `ithub` in `github.com`.
- **1–2 character queries**: trigram can't match these, so the search module falls back to `plain_text LIKE '%q%'` over the most recent 5,000 clips. That's fast enough and covers short searches.
- **Ranking**: we order by `bm25(clips_fts, 1.0, 0.3)` (text matters more than the app name), then blend with recency, so a recent exact match beats an old one. The weights get tuned with real data.
- **Highlighting**: `snippet()` provides the match context shown in the list.
- **Index size**: trigram indexes are larger than word indexes (roughly 3× the text). With 64 KiB of `plain_text` per clip at most, this stays manageable. We'll measure it at 100k clips in the storage spike.

Example query (filters are optional):

```sql
SELECT c.id, c.kind, c.source_app_name, c.last_used_at,
       snippet(clips_fts, 0, '[', ']', '…', 12) AS snippet
FROM clips_fts
JOIN clips c ON c.id = clips_fts.rowid
WHERE clips_fts MATCH :q
  AND (:kind IS NULL OR c.kind = :kind)
  AND (:app  IS NULL OR c.source_app_id = :app)
ORDER BY bm25(clips_fts, 1.0, 0.3) - (c.last_used_at / 8.64e10) -- recency boost, tuned later
LIMIT 50 OFFSET :offset;
```

The user's text is always escaped into an FTS5 string literal (wrapped in double quotes, with inner quotes doubled). It's never passed through as raw query syntax.

## Indexes

```sql
CREATE INDEX clips_by_recent  ON clips (last_used_at DESC);
CREATE INDEX clips_by_kind    ON clips (kind, last_used_at DESC);
CREATE INDEX clips_by_app     ON clips (source_app_id, last_used_at DESC);
CREATE INDEX clips_pinned     ON clips (last_used_at DESC) WHERE pinned = 1;
CREATE INDEX clip_formats_blob ON clip_formats (blob_hash) WHERE blob_hash IS NOT NULL;
CREATE INDEX pinboard_items_by_clip ON pinboard_items (clip_id);
```

`content_hash` and `uuid` are indexed through their `UNIQUE` constraints, and `clip_formats(clip_id, …)` through its `UNIQUE` constraint.

## Duplicates

Copying something that's already in the history moves it to the top instead of adding a second row.

1. The classifier computes `content_hash = BLAKE3(kind ‖ 0x00 ‖ canonical payload)`:

   | Kind | Canonical payload |
   | --- | --- |
   | `text`, `rich_text`, `code`, `link` | The full plain text, byte for byte. Whitespace isn't trimmed, because it matters in code. |
   | `color` | Normalised RGBA hex |
   | `image` | Bytes of the PNG representation, or TIFF/DIB if there's no PNG |
   | `file` | The file paths, sorted and joined with `\n` |

2. `INSERT … ON CONFLICT(content_hash) DO UPDATE SET last_used_at = :now, use_count = use_count + 1, source_app_* = excluded.source_app_*`.
3. For `rich_text`, the stored formats are replaced with the latest copy's. The text is the same, but the formatting may have changed.
4. Pasting a clip from Kopi does the same bump, and the watcher recognises Kopi's own write, so it doesn't create a duplicate.

The kind is part of the hash. So the same words copied as plain text and as rich text become two clips. That's intentional: they paste differently.

## Retention and cleanup

| Setting | Default | Notes |
| --- | --- | --- |
| `history.max_items` | unlimited | "Unlimited history" is a headline feature |
| `history.max_age_days` | unlimited | Options: 1, 7, 30, 90, 365 days |
| `history.max_total_bytes` | 2 GiB | Counts DB rows plus blob files |
| `limits.max_clip_bytes` | 50 MiB | Larger copies are skipped, with a quiet notice in the panel |
| `limits.max_text_index_bytes` | 64 KiB | Truncation point for `plain_text` |

Rules:

- **Pinned clips and clips on any pinboard are never removed by limits.** They're still removed by an explicit delete or by "Clear history" if the user chooses to include them.
- When a limit is exceeded, the oldest clips are deleted first, ordered by `last_used_at`.
- Retention runs at startup, at most once a minute after new clips arrive, and immediately when a limit is lowered.
- Deletes happen in one transaction. Blob and thumbnail files are removed after the commit, if no other row references them.
- **Orphan sweep** at startup: files in `blobs/` or `thumbs/` that no row references are deleted. This covers crashes between writing a file and committing.
- After "Clear history", Kopi runs `VACUUM` and a WAL checkpoint (`TRUNCATE`), so deleted content doesn't linger in free pages or the WAL file.

## Images and thumbnails

The original image bytes are stored once, in `clip_formats`: inline if they're 64 KiB or less, otherwise as a blob file. SQLite's own measurements put the crossover where external files become faster at about 100 KB ([source](https://www.sqlite.org/intern-v-extern-blob.html)), and keeping large data out of the database keeps its pages dense for list and search queries.

A thumbnail (PNG, longest edge 320 px) is generated off the main thread after insert. The list only ever loads thumbnails, through `kopi://thumb/<hash>`. The full image is loaded only for the preview pane or a paste.

Very large images (over `limits.max_clip_bytes`) are skipped, not downscaled. A clipboard manager that silently changes images would be surprising.

## Sync later

Nothing in the alpha syncs. A few choices keep sync possible without a schema rewrite: UUIDv7 `uuid` on synced entities, content hashes for dedupe across devices, and `updated_at` on pinboards. Tombstones and per-device clocks will come with the sync design.
