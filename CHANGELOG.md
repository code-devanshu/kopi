# Changelog

All notable changes to Kopi are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org).

## [Unreleased]

## [0.1.0-alpha.11] - 2026-10-23

Testers on an earlier alpha get this release as an update. After it, an earlier alpha can't open your history, so reinstalling one by hand starts it with an error.

### Added

- **Find text in images.** Kopi reads the text in the images you copy, on your computer with the text recognition built into macOS and Windows, so searching for a few words of an error message finds the screenshot it was in. Nothing is sent anywhere.
  - Image rows show the matching text during a search. The preview shows all the text read from the image, with **Copy text**, and the clip menu (⌘K) has **Copy Text in Image**.
  - Images already in your history are read once, in the background, newest first, and not while Low Power Mode (macOS) or Energy saver (Windows) is on.
  - On Windows it needs a language with Optical character recognition installed, which most languages have. When none is, Settings → Capture says how to add one.
  - Turn it off in Settings → Capture → **Find text in images**, which also removes the text it read.
  - An image still pastes as the image, with Return and ⌥Return. The text read from it isn't synced: each computer reads its own images.

### Changed

- The website is redesigned around screenshots of the app.

## [0.1.0-alpha.10] - 2026-10-21

Testers on an earlier alpha get this release as an update. On macOS, restart Kopi once after it updates to this release, or pasting only copies until you do. Later updates don't need that.

### Fixed

- On macOS, Kopi stopped pasting after it updated itself, and asked for Accessibility even though it was allowed, until you quit and reopened it. macOS stops trusting an app whose files are replaced while it runs, and Kopi installed updates as soon as they downloaded. Updates now download in the background and install when you choose **Restart to Update** or quit Kopi. On Windows, updates still install with **Restart to Update**.

## [0.1.0-alpha.9] - 2026-10-21

Testers on an earlier alpha get this release as an update.

### Added

- **Snippets.** Text you paste often, such as a reply, an address or a signature, written once in the new Snippets tab and pasted with Return like a clip.
  - Placeholders fill in when it's pasted: `{date}`, `{date:YYYY-MM-DD}`, `{time}`, `{datetime}`, `{clipboard}` (the newest text in your history), `{input:Name}` (asks for a Name first) and `{cursor}` (where the cursor ends up). The editor's Insert menu adds them.
  - Give a snippet a keyword, such as `sig`, and typing it in the search box puts that snippet first, in History too.
  - ⌘N makes a new snippet and ⌘E edits the selected one. Changes save as you type.
  - Save any text clip as a snippet from its menu, or by dragging it onto the Snippets tab.
  - A pasted snippet isn't added to your history.
  - Snippets sync between your computers, with an option in Settings → Sync → What to sync. A computer that updates gets every snippet the others have, including ones made before it updated.

### Changed

- The website shows snippets, in a feature of their own and in the examples for writers. The "Snippets" pinboard in the developer example is now "Commands".

## [0.1.0-alpha.8] - 2026-10-20

Testers on an earlier alpha get this release as an update.

### Added

- **Sync between your computers.** Settings → Sync syncs your history, pins and pinboards between your Macs and Windows PCs, in any mix, directly over your local network. Clips go straight from one computer to the other, end-to-end encrypted, and never through a server. It's off until you turn it on.
  - Pair a computer once with a 6-digit code. To add a third, pair it with any one of the others.
  - Text, rich text, links, code, colors and images sync, and you can choose which. File clips stay on the computer they were copied on.
  - Deleting a clip, or clearing history, deletes it on every computer. Each keeps its own history limits, settings and excluded apps.
  - A clip from another computer shows where it was copied in the preview.
  - On a Mac, turning sync on makes macOS ask whether Kopi may find devices on your local network. On Windows, Windows Defender Firewall asks to allow Kopi. See [INSTALL_ALPHA.md](docs/INSTALL_ALPHA.md#sync).

### Changed

- On Windows, copies an app marks as not for other devices (`CanUploadToCloudClipboard`) are kept in your history but never synced.
- The website and the README say sync is here, over your local network, instead of coming after 1.0.

## [0.1.0-alpha.7] - 2026-10-19

Testers on an earlier alpha get this release as an update.

### Changed

- On Windows, onboarding and Settings warn when the shortcut is Ctrl+Shift+V. Browsers and many other apps use it to paste without formatting, and Kopi takes it from them while it runs. Ctrl+Shift+V stays the default, and you can choose another shortcut in either place.
- The website's download buttons download the newest `.dmg` or Windows installer straight away, instead of opening GitHub's releases page, and show how to install it. The hero offers macOS and Windows side by side, with your system's first.
- The website no longer suggests that sync works today. It says sync is coming after 1.0.

## [0.1.0-alpha.6] - 2026-10-16

Testers on an earlier alpha get this release as an update.

### Fixed

- On macOS, **Check for Updates…** seemed to do nothing when another app was full screen or you were on another desktop: its result opened on a different Space, and Kopi waited for an answer you couldn't see. Kopi's alerts now open on the Space you're on, and choosing **Check for Updates…** again brings an open one forward.
- While Kopi checks for an update, **Check for Updates…** in the menu bar shows what it's doing, as **Checking for Updates…**, **Downloading Update… 45%** and, on macOS, **Installing Update…**, and can't be chosen again until it's done.

## [0.1.0-alpha.5] - 2026-10-16

Testers on an earlier alpha get this release as an update.

### Fixed

- On macOS, Kopi sometimes couldn't paste after it opened at login, even with Accessibility shown as allowed, until you quit and reopened it. Launch at login now adds Kopi to System Settings → General → Login Items, so macOS opens it like any other app. If you had launch at login on, Kopi moves it there the first time it starts and removes the old `~/Library/LaunchAgents/Kopi.plist`.

## [0.1.0-alpha.4] - 2026-10-15

Kopi comes to Windows. Testers on an earlier alpha get this release as an update.

### Added

- Kopi for Windows 10 and 11, a first build for testers. It captures copies, opens with Ctrl+Shift+V, pastes into the app you were in and updates itself. The shared code reads Windows clipboard formats, Ctrl stands in for ⌘ in the panel, and password managers are excluded by default. The installer isn't signed yet, so Windows asks you to confirm it the first time. See [Installing Kopi](docs/INSTALL_ALPHA.md#windows).
- CI checks and builds the desktop app on Windows too, and the Release workflow builds a Windows installer next to the `.dmg`.
- The website and the README offer the Windows download next to the macOS one. The waitlist is now for the beta and sync.

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

[Unreleased]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.11...HEAD
[0.1.0-alpha.11]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.10...v0.1.0-alpha.11
[0.1.0-alpha.10]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.9...v0.1.0-alpha.10
[0.1.0-alpha.9]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.8...v0.1.0-alpha.9
[0.1.0-alpha.8]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.7...v0.1.0-alpha.8
[0.1.0-alpha.7]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.6...v0.1.0-alpha.7
[0.1.0-alpha.6]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.5...v0.1.0-alpha.6
[0.1.0-alpha.5]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.4...v0.1.0-alpha.5
[0.1.0-alpha.4]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.3...v0.1.0-alpha.4
[0.1.0-alpha.3]: https://github.com/code-devanshu/kopi/compare/v0.1.0-alpha.2...v0.1.0-alpha.3
[0.1.0-alpha.2]: https://github.com/code-devanshu/kopi/releases/tag/v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/code-devanshu/kopi/releases/tag/v0.1.0-alpha.1
