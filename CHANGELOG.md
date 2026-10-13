# Changelog

All notable changes to Kopi are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions will follow [Semantic Versioning](https://semver.org) once the first release ships.

## [Unreleased]

### Added

- "Coming soon" website with a waitlist at [kopi.devanshuverma.in](https://kopi.devanshuverma.in).
- pnpm monorepo, with the website in `apps/web`.
- MIT License, contributing guide, code of conduct and security policy.
- Roadmap and a draft architecture doc.
- Issue templates, a pull request template and CI for the website.
- Desktop app scaffold in `apps/desktop` (Tauri 2, React, TypeScript), with a menu bar icon, single-instance and launch-at-login plugins, and TypeScript bindings generated from the Rust commands.
- Rust workspace with the `kopi-core` and `kopi-platform` crates.
- CI for the desktop app on macOS: rustfmt, Clippy, tests, a bindings check, lint and an app build.
- Storage in `kopi-core`: SQLite (WAL) with migrations, the full schema and FTS5 trigram search index, upsert by content hash, content-addressed blob files for data over 64 KiB, and PNG thumbnails made on a background thread. Includes a 100k-clip benchmark.
- macOS clipboard watcher: `Pasteboard`, `ForegroundApp` and `UserIdle` traits in `kopi-platform` with an AppKit implementation, and a watcher thread in `kopi-core` that polls `changeCount` (250 ms, 500 ms when idle), reads format names before any content, checks the pasteboard access grant, takes the source app from `org.nspasteboard.source` or the frontmost app, ignores Kopi's own writes, and reads only the captured formats from every item. `cargo run -p kopi-core --example watch` shows it working.
- Privacy filter in `kopi-core`, in two stages. Before any read: concealed, transient and auto-generated markers (including the legacy ones), excluded apps, pause (indefinite or timed) and kinds turned off. After the read, in memory: the size limit and kinds known only from the content. Password managers and Keychain Access are excluded by default, seeded once so a removed default stays removed, and the store can list, add and remove excluded apps.
- Classifier in `kopi-core`: picks the kind of each copy (text, rich text, link, image, file, color or code), cuts the plain text to 64 KiB, extracts metadata (image size, color, link host, file names, code language), and computes the BLAKE3 content hash, so copying something again moves it to the top. Colors are read from CSS text and from macOS color archives. The `watch` example now stores what it sees and shows duplicates.
- Search in `kopi-core` and the `search_clips` command: words of 3+ characters match anywhere through the trigram index (case and accents aside), shorter ones through `LIKE`, with filters for kind, source app and pinboard. Results are ranked by length and recency, paged, and come with the matches marked. At 100k clips, searches take 3 ms at the median and stay under 50 ms with filters.
- The panel: ⇧⌘V opens a non-activating panel over the app you're in, on the screen with the pointer, and Esc or clicking elsewhere hides it. It shows the history in a virtualised list with thumbnails, kind icons, source app and age, searches as you type with the matches marked, filters by kind and app, and has a preview pane (Space or ⌘Y). Arrows, Page Up/Down, ⌘↑/⌘↓, ⌘⌫ (delete) and Esc work from the keyboard. Return, ⌥Return and ⌘1–9 are bound for the paste engine. Follows light and dark mode and reduced motion.
- The app now captures copies on macOS while it runs, and the panel updates live from `clip:added`, `clip:updated`, `clip:removed` and `thumbnail:ready` events. Thumbnails and images reach the panel through a `kopi://` protocol. New commands: `get_clip`, `delete_clips`, `source_apps` and `hide_panel`.
- Release workflow: a version tag builds a universal macOS `.dmg` and attaches it to a draft GitHub Release. Releases are signed with a self-signed certificate, so testers keep their permissions across updates, and aren't notarized ([ADR 0006](docs/decisions/0006-macos-distribution-without-notarization.md)). Install steps for testers are in `docs/INSTALL_ALPHA.md`.

[Unreleased]: https://github.com/code-devanshu/kopi/commits/main
