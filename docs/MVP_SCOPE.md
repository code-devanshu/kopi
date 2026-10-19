# MVP scope: macOS alpha

The macOS alpha is the first build we hand to testers. Its job is to prove that the core loop (copy, find, paste) is fast, reliable and private on one platform before we build the second. Anything not listed under "In scope" is out.

## Target

- **macOS 14 Sonoma and later**, Apple silicon and Intel (universal build). It's tested on the minimum (macOS 14), the current release (macOS 27) and the one before (macOS 26).
- Distributed as a notarized `.dmg` from GitHub Releases. If the Apple Developer Program account isn't ready yet, testers get an ad-hoc-signed build with install instructions.
- Testers are people from the waitlist who said they'd like to test.

## In scope

| Feature | Done when |
| --- | --- |
| **History** | Every copy of a supported kind is captured within 0.5 s and shown newest first. Copying something already in history moves it to the top (no duplicates). History survives restarts and is unlimited by default. |
| **Search** | Typing filters as you type (under 50 ms per keystroke at 100k clips on an M1). Substring matching, filters by kind and source app, and the match is highlighted. |
| **Global shortcut panel** | ⇧⌘V (configurable) opens a borderless panel on the screen with the cursor, over full-screen apps, without a Dock icon. Esc or clicking elsewhere hides it. Fully keyboard-driven: arrows, Return, ⌘1–9 for quick pick, ⌘⌫ to delete, Space to preview. |
| **Paste into the active app** | Return pastes into the app that was frontmost. ⌥Return pastes as plain text. Without Accessibility permission, it copies and shows "Press ⌘V". |
| **Clip kinds** | `text`, `rich_text`, `link`, `image`, `file` and `color` are captured, previewed and pasted with their original formats. Images get thumbnails, links show their host, and colors show a swatch. `code` is detected with a simple heuristic and shown in a monospace font. |
| **Pin + pinboards** | Pin or unpin a clip. Create, rename, recolor, reorder and delete pinboards, and add or remove clips. Pinned and pinboard clips are never removed by limits. |
| **Privacy filter** | Concealed, transient and auto-generated content is never read or stored. Pause capture from the tray or the panel, indefinitely or for 5 minutes or 1 hour. See [PRIVACY_DESIGN.md](PRIVACY_DESIGN.md). |
| **Excluded apps** | Seeded with common password managers. Add an app from a picker or from a clip ("Don't capture from this app"), and remove apps. |
| **Settings** | Shortcut, history limits (count, age, size), kinds to capture, excluded apps, launch at login, theme (system, light, dark), clear history (everything, the last hour or the last day), and permission status with "Open System Settings" buttons. |
| **Launch at login** | A toggle in Settings and onboarding, using `SMAppService` (login item). |
| **Tray icon** | Menu bar icon (template image) with: Open Kopi, Pause/Resume capture, Settings, About, Quit. |
| **Onboarding** | First-run window that explains and requests Accessibility, sets the shortcut, and offers launch at login. It also explains and requests Paste from Other Apps, but only on macOS versions that enforce it (see [PLATFORM_NOTES.md](PLATFORM_NOTES.md#pasteboard-privacy-affects-the-whole-app)). |

### Quality bar

- Idle CPU under 0.5% and memory under 150 MB with 10k clips (webview included). No visible delay opening the panel.
- No clip content in logs. No network calls apart from the updater.
- The Rust core has unit tests for the privacy filter, classifier, dedupe, retention and search. Storage is tested against a temporary database.
- The CI workflow builds the macOS app and runs `cargo test`, `cargo clippy` and the UI lint.

## Explicitly not in the alpha

| Not included | Why / when |
| --- | --- |
| **Sync** (Mac ↔ Windows or Mac ↔ Mac) | Designed in [SYNC_DESIGN.md](SYNC_DESIGN.md): end-to-end encrypted, over the local network ([ADR 0007](decisions/0007-lan-sync.md)). Ships in a later alpha. |
| **Windows** | Next phase ("Windows alpha"). The platform traits are designed for it now. |
| **Snippets with variables** (templates, `{date}`, cursor placement) | Later. Pinboards cover saved text for now. |
| **OCR** (searching text inside images) | Later. Possible with Vision on macOS. |
| **Mobile** (iOS, Android) | Not planned |
| Linux | Not planned for 1.0 |
| Encrypted local database | Open question for the beta |
| Auto-updates | In the macOS alpha (`tauri-plugin-updater`, [#38](https://github.com/code-devanshu/kopi/issues/38)), so testers don't download and approve every build. Windows comes with Windows signing. |
| Restoring the previous clipboard after paste | Later setting |
| Editing clips, merging clips, paste stack / sequential paste | Later |
| Secret detection by pattern (API keys, cards) | Later, opt-in |
| Mac App Store build | Not possible with Accessibility and background clipboard reads |
| Localization | English only for the alpha |
