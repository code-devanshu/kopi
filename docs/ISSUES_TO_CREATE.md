# Issues to create: macOS alpha

These are the GitHub issues for the macOS alpha ([MVP_SCOPE.md](MVP_SCOPE.md)), in a sensible order. Start with the three spikes: they can change the design. All of them now exist on GitHub, and each heading links to its issue. The issues are the source of truth from here on; this file is kept as a record.

## Labels

| Label | Color | Description |
| --- | --- | --- |
| `area: desktop` | `#1d76db` | The desktop app (`apps/desktop`, `crates/`) |
| `platform: macos` | `#5319e7` | macOS-specific work |
| `type: spike` | `#fbca04` | Time-boxed research with a written result |
| `type: feature` | `#0e8a16` | New functionality |
| `privacy` | `#b60205` | Affects what is captured, stored or shown |
| `milestone: mac-alpha` | `#c5def5` | Needed for the macOS alpha (or use a GitHub milestone instead) |

`enhancement`, `bug`, `good first issue` and `help wanted` already exist or are GitHub defaults.

---

### 1. Spike: pasteboard privacy on macOS 27.2+ ([#4](https://github.com/code-devanshu/kopi/issues/4))

**Labels:** `area: desktop`, `platform: macos`, `type: spike`, `privacy`

macOS 27.2 removes the pasteboard-privacy developer flag, and background clipboard reads may now prompt by default ([PLATFORM_NOTES.md](PLATFORM_NOTES.md#pasteboard-privacy-affects-the-whole-app)). Build a tiny signed test app that polls `changeCount`, reads `types`, then reads content, and record the results:
- Which calls trigger the alert?
- How does `accessBehavior` change?
- What does a denied read return?
- Does "Allow" survive a rebuild or an app update?

Test on 27.2+, on 15.4+ (where `accessBehavior` first exists) and on 14, the minimum, which has no `accessBehavior`, to confirm that reads don't prompt there. Write up the results in PLATFORM_NOTES and update ADR 0003.

### 2. Spike: non-activating panel with `tauri-nspanel` ([#5](https://github.com/code-devanshu/kopi/issues/5))

**Labels:** `area: desktop`, `platform: macos`, `type: spike`

Prove the panel window:
- borderless, on the screen with the cursor, over full-screen apps
- no Dock icon (`Accessory` policy)
- keyboard focus without activating Kopi
- hides on blur and on Esc

Use Tauri 2.12 and `tauri-nspanel` 2.1. Document the setup and any gotchas, such as webview focus or IME.

### 3. Spike: paste into the active app on macOS ([#6](https://github.com/code-devanshu/kopi/issues/6))

**Labels:** `area: desktop`, `platform: macos`, `type: spike`

Write a clip to the pasteboard, hide the panel and post ⌘V with `CGEvent`. Cover:
- the Accessibility check and prompt
- a layout-aware V key code (Dvorak, "Dvorak – QWERTY ⌘")
- modifiers still held from the hotkey
- the delay needed

Test in TextEdit, Safari, Chrome, VS Code, Terminal, Slack and Notes, and record what fails.

### 4. Scaffold `apps/desktop` and the Rust workspace ([#7](https://github.com/code-devanshu/kopi/issues/7))

**Labels:** `area: desktop`, `type: feature`

Create the following:
- `@kopi/desktop` (Tauri 2.12, React, TypeScript, Vite) and a root Cargo workspace with `crates/kopi-core` and `crates/kopi-platform`, matching the folder layout in [ARCHITECTURE.md](ARCHITECTURE.md#planned-folder-structure)
- bundle ID `in.devanshuverma.kopi`
- tray icon with Quit
- the `single-instance` and `autostart` plugins wired up
- typed IPC bindings (choose `specta` or `ts-rs`)
- CI: macOS build, `cargo test`, `cargo clippy -D warnings` and the UI lint

Add `dev:desktop` and `build:desktop` scripts at the root.

### 5. Storage: schema, migrations and blob store ([#8](https://github.com/code-devanshu/kopi/issues/8))

**Labels:** `area: desktop`, `type: feature`

Implement [DATA_MODEL.md](DATA_MODEL.md) in `kopi-core`:
- `rusqlite` (bundled), WAL, pragmas, migration runner
- all tables and FTS triggers
- content-addressed blob store and PNG thumbnails off the main thread
- upsert-by-hash

Benchmark insert, search and startup at 100k clips, and record the numbers in the PR.

### 6. macOS clipboard watcher ([#9](https://github.com/code-devanshu/kopi/issues/9))

**Labels:** `area: desktop`, `platform: macos`, `type: feature`, `privacy`

Build the `Pasteboard` and `ForegroundApp` traits and their macOS implementation with `objc2-app-kit`:
- adaptive `changeCount` polling
- `types()` before any read
- reading all pasteboard items and the formats on the allow-list
- source app from `org.nspasteboard.source` or `frontmostApplication`
- ignoring Kopi's own writes
- exposing `accessBehavior`

Depends on [#4](https://github.com/code-devanshu/kopi/issues/4).

### 7. Privacy filter and excluded apps ([#10](https://github.com/code-devanshu/kopi/issues/10))

**Labels:** `area: desktop`, `type: feature`, `privacy`, `good first issue`

This is a pure-Rust filter. It has two stages:
- **before any read**, from the format list, source app and settings: concealed, transient and auto-generated markers, the legacy markers, excluded apps, paused state, and kinds known from the format list
- **after the read, in memory**: the size limit and kinds known only from the content (link, code, color). Failing content is dropped without being stored or logged.

Seed the default excluded apps and verify their bundle IDs. Write unit tests for every rule in [PRIVACY_DESIGN.md](PRIVACY_DESIGN.md).

### 8. Classifier and duplicate handling ([#11](https://github.com/code-devanshu/kopi/issues/11))

**Labels:** `area: desktop`, `type: feature`, `good first issue`

Turn raw formats into a clip:
- choose the kind (text, rich text, link, image, file, color, code)
- extract `plain_text`, truncated to 64 KiB
- extract metadata (image size, color value, link host, file names)
- compute the BLAKE3 content hash as specified in DATA_MODEL

Include table-driven tests, especially for links, colors and code detection.

### 9. Search ([#12](https://github.com/code-devanshu/kopi/issues/12))

**Labels:** `area: desktop`, `type: feature`

Implement the `search_clips` command:
- safe FTS5 query building (escaped literals)
- trigram MATCH and the `LIKE` fallback for 1–2 characters
- filters for kind, source app and pinboard
- bm25 blended with recency
- `snippet()` highlights and paging

Target: under 50 ms per keystroke at 100k clips.

### 10. Panel UI: history, search and preview ([#13](https://github.com/code-devanshu/kopi/issues/13))

**Labels:** `area: desktop`, `type: feature`

Build the React panel:
- search box with focus on open
- virtualised list with thumbnails, kind icons, source app and relative time
- preview pane, with Space to toggle
- kind and app filters, empty and loading states
- live updates from `clip:*` events
- full keyboard control (arrows, Return, ⌥Return, ⌘1–9, ⌘⌫, Esc)

Respect light and dark mode and reduced motion.

### 11. Paste engine ([#14](https://github.com/code-devanshu/kopi/issues/14))

**Labels:** `area: desktop`, `platform: macos`, `type: feature`

Implement `paste_clip` and `copy_clip` from [ADR 0004](decisions/0004-paste-simulation-approach.md):
- write all stored formats, or plain text only
- mark the write as Kopi's own
- hide the panel and post ⌘V when Accessibility is granted, otherwise copy and show a toast
- bump `last_used_at` and `use_count`

Depends on [#6](https://github.com/code-devanshu/kopi/issues/6).

### 12. Pins and pinboards ([#15](https://github.com/code-devanshu/kopi/issues/15))

**Labels:** `area: desktop`, `type: feature`

Core and UI for:
- pinning and unpinning clips
- creating, renaming, recoloring, reordering and deleting pinboards
- adding and removing clips by drag or menu
- a pinboard sidebar or switcher in the panel

Pinned and pinboard clips must be exempt from retention.

### 13. Settings window and onboarding ([#16](https://github.com/code-devanshu/kopi/issues/16))

**Labels:** `area: desktop`, `platform: macos`, `type: feature`, `privacy`

**Settings:**
- shortcut recorder
- history limits, kinds to capture, excluded apps
- launch at login, theme, clear history
- permission status with "Open System Settings" buttons

**First-run onboarding:**
- explain and request Accessibility, plus Paste from Other Apps on macOS versions that enforce it (skip that step elsewhere)
- choose the shortcut
- launch at login

### 14. Retention, clear history and cleanup ([#17](https://github.com/code-devanshu/kopi/issues/17))

**Labels:** `area: desktop`, `type: feature`, `privacy`

Covers:
- count, age and size limits (pinned and pinboard clips are exempt)
- debounced runs, plus an immediate run when a limit is lowered
- orphan blob and thumbnail sweep at startup
- "Clear history" and "Clear last hour/day", followed by `VACUUM` and a WAL checkpoint
- FTS `secure-delete`

Include tests for each rule in [DATA_MODEL.md](DATA_MODEL.md#retention-and-cleanup).

### 15. Alpha release pipeline ([#18](https://github.com/code-devanshu/kopi/issues/18))

**Labels:** `area: desktop`, `platform: macos`

Set up a GitHub Actions workflow that builds a universal `.dmg` when a tag is pushed, with:
- Developer ID signing and notarization through Tauri's bundler (secrets in the repo settings), with an ad-hoc fallback
- upload to a draft GitHub Release

Document the install steps for testers, including how to grant the permissions.
