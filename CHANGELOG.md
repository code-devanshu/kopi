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

[Unreleased]: https://github.com/code-devanshu/kopi/commits/main
