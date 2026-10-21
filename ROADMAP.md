# Roadmap

This is the plan for getting Kopi from an idea to a stable app on macOS and Windows. There are no dates yet. Each phase ships when it's solid, not by a deadline.

**Status key:** ✅ done · 🔨 in progress · ⏳ planned

Want to help with a phase? Look for matching [issues](https://github.com/code-devanshu/kopi/issues), or start a [discussion](https://github.com/code-devanshu/kopi/discussions).

---

## 1. Website and waitlist ✅

A public home for the project and a way for people to follow along.

- ✅ Website at [kopi.devanshuverma.in](https://kopi.devanshuverma.in), now with the macOS and Windows downloads
- ✅ Waitlist form with spam protection
- ✅ pnpm monorepo, ready for the desktop app
- ✅ Open-source setup: license, contributing guide and issue templates

## 2. Design and architecture ✅

Decide what Kopi looks like and how it's built before writing the app.

- ✅ Research macOS, Windows and Tauri ([docs/PLATFORM_NOTES.md](docs/PLATFORM_NOTES.md))
- ✅ Architecture ([docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)) and decision records ([docs/decisions/](docs/decisions/))
- ✅ Data model and search ([docs/DATA_MODEL.md](docs/DATA_MODEL.md))
- ✅ Privacy design ([docs/PRIVACY_DESIGN.md](docs/PRIVACY_DESIGN.md))
- ✅ macOS alpha scope ([docs/MVP_SCOPE.md](docs/MVP_SCOPE.md))
- ✅ Design the panel: history list, search, filters and preview
- ✅ Design pinboards, settings and onboarding
- ✅ Define the keyboard shortcuts
- ✅ Spike the risky parts on macOS: pasteboard privacy, the panel window and pasting into the active app ([docs/spikes/](docs/spikes/))

## 3. macOS alpha 🔨

Usable builds for early testers on macOS, out now on [GitHub Releases](https://github.com/code-devanshu/kopi/releases). The scope is in [docs/MVP_SCOPE.md](docs/MVP_SCOPE.md), and each release is in [CHANGELOG.md](CHANGELOG.md).

- ✅ Tauri app in `apps/desktop`
- ✅ Clipboard watcher for text, rich text, links, images, files, colors and code
- ✅ Local history stored in SQLite, with limits by count, age and size
- ✅ Full-text search with kind and source-app filters
- ✅ Global hotkey, keyboard navigation and paste into the active app
- ✅ Pins and pinboards
- ✅ Snippets with placeholders, in 0.1.0-alpha.9 ([docs/SNIPPETS_DESIGN.md](docs/SNIPPETS_DESIGN.md))
- ✅ Skip copies from password managers and content marked as concealed, plus excluded apps
- ✅ Settings, onboarding, menu bar icon and launch at login
- ✅ Signed releases and automatic updates, without paying for notarization ([ADR 0006](docs/decisions/0006-macos-distribution-without-notarization.md))
- ✅ An easier first install: a one-line install from Terminal, and a `.dmg` that shows the steps ([#39](https://github.com/code-devanshu/kopi/issues/39))
- ⏳ Retest pasteboard privacy on macOS 27.2 ([#21](https://github.com/code-devanshu/kopi/issues/21))
- ⏳ Fixes from testers' reports

## 4. Windows alpha 🔨

The same app, built for Windows 10 and 11, out now on [GitHub Releases](https://github.com/code-devanshu/kopi/releases).

- ✅ Windows clipboard watcher, hotkey and paste
- ✅ Source-app detection on Windows
- ✅ Tray icon and launch at login
- ✅ Windows installer and automatic updates, built by CI
- 🔨 Feature parity with the macOS alpha, checked by testers
- ⏳ Fixes from testers' reports

## 5. Private beta ⏳

A wider group of testers on both platforms, with the features rounded out.

- ⏳ Feedback from the alphas on pinboards, settings and search
- ⏳ Settle the open questions in the design docs with contributors
- ⏳ Decide on encrypting the local database
- ⏳ Windows code signing and automatic updates
- ⏳ Performance work on large histories
- ⏳ Accessibility pass

## 6. v1.0 public launch ⏳

A stable release on macOS and Windows that anyone can download for free.

- ✅ Downloads on the website and GitHub Releases
- ✅ Release process and changelog ([docs/RELEASING.md](docs/RELEASING.md))
- ⏳ User docs and FAQ
- ⏳ Security review of local storage

## 7. Encrypted sync 🔨

Optional sync between your Mac and Windows computers, directly over your local network, in the alpha since 0.1.0-alpha.8.

- ✅ The protocol and encryption, designed in the open ([docs/SYNC_DESIGN.md](docs/SYNC_DESIGN.md), [ADR 0007](docs/decisions/0007-lan-sync.md))
- ✅ End-to-end encryption, with keys that never leave your computers
- ✅ Pairing with a code
- ✅ Conflict handling, deletes, and sync for pins, pinboards and snippets
- 🔨 Testing on real networks, Mac and Windows, with testers
- ⏳ Sync away from home, through a relay that can't read your clips, in its own design

---

The roadmap will change as we learn. If you think something is missing or in the wrong order, open a discussion.
