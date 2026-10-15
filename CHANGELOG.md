# Changelog

All notable changes to Kopi are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org).

## [Unreleased]

## [0.1.0-alpha.3] - 2026-10-15

Testers on alpha.2 get this release as an update.

### Added

- A one-line install from Terminal: `curl -fsSL https://kopi.devanshuverma.in/install.sh | bash`. macOS doesn't block Kopi installed this way, so there's no **Open Anyway** step. The script installs the newest release, and only if it's signed with Kopi's certificate. Run it again to update by hand.
- The `.dmg` window shows the drag to Applications and how to open Kopi the first time. Builds from GitHub Actions had no layout before.
- The website shows the install command, with a copy button, under the download button.

### Fixed

- When Kopi can't paste, the panel says to press ⌘V and how to allow Kopi under Privacy & Security → Accessibility, with a button that opens it. It covers Kopi missing from the list (add it with +) and Kopi on but still unable to paste (remove it with −, then add it again). The Accessibility row in Settings gives the same steps.
- The website no longer scrolls sideways on phones.
- The website's footer says the macOS alpha is out, instead of "Brewing now", and no longer links to an X account that doesn't exist.

## [0.1.0-alpha.2] - 2026-10-13

Kopi goes public. Testers on alpha.1 get this release as an update.

### Added

- The website's download button links to [GitHub Releases](https://github.com/code-devanshu/kopi/releases), and the waitlist is now for Windows.
- The README shows the app and covers downloading, opening it the first time without notarization, and building from source.

### Fixed

- Update messages, such as "Kopi is up to date", open in front on the screen with the pointer, one at a time. They were system alerts on the main display, which may not be the screen you're looking at, and every click on **Check for Updates…** queued another.
- A development build no longer installs an update over its build folder. It says what it found instead.

## [0.1.0-alpha.1] - 2026-10-13

The first macOS alpha.

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
- Paste engine: Return pastes the selected clip into the app you were in, ⌥Return pastes it as plain text, ⌘1–9 paste the first nine clips, and ⌘C copies without pasting. Kopi writes the clip to the clipboard with every stored format and posts ⌘V. Without the Accessibility permission, the clip stays on the clipboard and the panel says to press ⌘V ([ADR 0004](docs/decisions/0004-paste-simulation-approach.md)).
- Pins and pinboards: ⌘P pins a clip, a switcher under the search box shows History, Pinned and each pinboard (⌘[ and ⌘] step through them), and a clip menu (right-click or ⌘K) pins, copies, deletes and adds a clip to boards. Boards can be renamed, recolored, reordered and deleted, and clips can be dragged onto them.
- History limits: by count, age and total size (2 GiB by default), applied in the background, oldest first. Pinned clips and clips on a pinboard are never removed. A sweep at startup removes files no clip uses. **Clear History** in the menu bar clears the last hour or day.
- Settings window (⌘,): the shortcut, launch at login, appearance, permission status, the kinds to capture, excluded apps, history limits, and clearing all history.
- First-run setup: a window that explains Kopi, asks for the missing permissions, sets the shortcut and launch at login, and opens again from **Set Up Kopi…** in the menu bar.
- Release workflow: a version tag builds a universal macOS `.dmg` and attaches it to a draft GitHub Release. Releases are signed with a self-signed certificate, so testers keep their permissions across updates, and aren't notarized ([ADR 0006](docs/decisions/0006-macos-distribution-without-notarization.md)). Install steps for testers are in `docs/INSTALL_ALPHA.md`.
- Automatic updates on macOS with the Tauri updater: Kopi checks GitHub a little after launch and every 6 hours, installs updates in the background and offers Restart to Update in the menu bar, with Check for Updates… there too and an **Update automatically** switch in Settings. Updates aren't quarantined, so Gatekeeper only blocks the first install. Publishing a release offers it to the app.

[Unreleased]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.3...HEAD
[0.1.0-alpha.3]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.2...v0.1.0-alpha.3
[0.1.0-alpha.2]: https://github.com/code-devanshu/kopi/releases/tag/v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/code-devanshu/kopi/releases/tag/v0.1.0-alpha.1
