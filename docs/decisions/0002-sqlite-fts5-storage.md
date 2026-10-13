# 0002. Store history in SQLite with FTS5, large data on disk

- **Status:** Accepted
- **Date:** 2026-10-11

## Context

Kopi keeps an unlimited history that can reach hundreds of thousands of clips. Search must keep up as the user types, including substring search on URLs and code. Images can be several megabytes. Everything stays local.

## Decision

- Use **SQLite** through **`rusqlite`** with the `bundled` feature, which gives SQLite 3.54 with FTS5 compiled in on every OS. The database lives in Rust, and the UI reaches it only through Tauri commands.
- Use an **FTS5 external-content table** with the **trigram tokenizer** over `plain_text` and `source_app_name`, kept in sync with triggers. The FTS5 `secure-delete` option is on. Queries under 3 characters fall back to `LIKE` on recent clips.
- Store format data of **64 KiB or less inline** as BLOBs. Larger data goes in **content-addressed files** (`blobs/<blake3>`), with PNG thumbnails in `thumbs/`.
- Run WAL mode with one writer thread and a small read pool.
- Store settings in a `settings` table rather than a separate JSON file, so a single file holds all state.

The full schema is in [DATA_MODEL.md](../DATA_MODEL.md).

## Alternatives considered

| Option | Why not |
| --- | --- |
| `tauri-plugin-sql` | Exposes SQL to the webview, which breaks our rule that the UI can't read clipboard data directly. It's also sqlx-based and async-first, which is more than we need. |
| `sqlx` in Rust | Fine, but compile-time query checking needs a database at build time, and async isn't needed for a local single-user DB. `rusqlite` is simpler. |
| FTS5 with `unicode61` (word) tokenizer | Smaller index, but can't find fragments such as `ithub` or the middle of an identifier. Prefix search alone isn't enough for clipboard content. |
| Tantivy or another search engine | A second store to keep consistent, and a bigger binary. FTS5 is enough at our scale. |
| All BLOBs in SQLite | Large images bloat the database and slow list queries. SQLite's own measurements favour external files above about 100 KB. |
| JSON files or sled/redb | No full-text search, and no relational pinboards. |

## Consequences

- **Good:** one dependency, one file, transactional, and fast enough for 100k+ clips. Easy to inspect while debugging.
- **Bad:** a trigram index is roughly 3× the indexed text, which we cap at 64 KiB per clip. Files and rows can drift after a crash, so we need an orphan sweep at startup.
- **Follow-up:** tune the ranking with real data. The 100k-clip benchmark (`crates/kopi-core/examples/storage_bench.rs`, results in PR #25) found that ranking every match costs 60 to 150 ms for very common terms, while newest-first reads straight off the index in under 20 ms. `bm25()` turned out to count every match before scoring any, whatever the `LIMIT`, so the search module (#12) ranks the newest 1,000 matches in Rust with bm25's length normalisation and recency instead. See [DATA_MODEL.md](../DATA_MODEL.md#full-text-search-fts5). Searches with a filter that rejects most matches still walk the whole index (up to about 45 ms in the benchmark). A slim covering index for the filters could cut that if real histories need it.
