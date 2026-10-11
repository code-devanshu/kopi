# Roadmap

This is the plan for getting Kopi from an idea to a stable app on macOS and Windows. There are no dates yet. Each phase ships when it's solid, not by a deadline.

**Status key:** ✅ done · 🔨 in progress · ⏳ planned

Want to help with a phase? Look for matching [issues](https://github.com/code-devanshu/kopi/issues), or start a [discussion](https://github.com/code-devanshu/kopi/discussions).

---

## 1. Website and waitlist ✅

A public home for the project and a way for people to follow along.

- ✅ "Coming soon" website at [kopi.devanshuverma.in](https://kopi.devanshuverma.in)
- ✅ Waitlist form with spam protection
- ✅ pnpm monorepo, ready for the desktop app
- ✅ Open-source setup: license, contributing guide and issue templates

## 2. Design and architecture 🔨

Decide what Kopi looks like and how it's built before writing the app.

- ✅ Research macOS, Windows and Tauri ([docs/PLATFORM_NOTES.md](docs/PLATFORM_NOTES.md))
- 🔨 Architecture ([docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)) and decision records ([docs/decisions/](docs/decisions/))
- 🔨 Data model and search ([docs/DATA_MODEL.md](docs/DATA_MODEL.md))
- 🔨 Privacy design ([docs/PRIVACY_DESIGN.md](docs/PRIVACY_DESIGN.md))
- 🔨 macOS alpha scope ([docs/MVP_SCOPE.md](docs/MVP_SCOPE.md))
- ⏳ Settle the open questions in those docs with contributors
- ⏳ Design the main window: history list, search, filters and preview
- ⏳ Design pinboards, settings and onboarding
- ⏳ Define the keyboard shortcuts
- ⏳ Spike the risky parts on macOS: pasteboard privacy on macOS 27.2+, the panel window and pasting into the active app

## 3. macOS alpha ⏳

First usable builds for early testers on macOS. The full scope is in [docs/MVP_SCOPE.md](docs/MVP_SCOPE.md).

- ⏳ Tauri app scaffold in `apps/desktop`
- ⏳ Clipboard watcher for text, rich text, links, images, files and colors
- ⏳ Local history stored in SQLite
- ⏳ Full-text search with type and source-app filters
- ⏳ Global hotkey, keyboard navigation and paste into the active app
- ⏳ Pins and pinboards
- ⏳ Skip copies from password managers and content marked as concealed, plus excluded apps
- ⏳ Settings, onboarding, menu bar icon and launch at login

## 4. Windows alpha ⏳

The same app, built for Windows.

- ⏳ Windows clipboard watcher, hotkey and paste
- ⏳ Source-app detection on Windows
- ⏳ Tray icon and launch at login
- ⏳ Feature parity with the macOS alpha

## 5. Private beta ⏳

A wider group of testers on both platforms, with the features rounded out.

- ⏳ Feedback from the alphas on pinboards, settings and search
- ⏳ Decide on encrypting the local database
- ⏳ Windows code signing and automatic updates
- ⏳ Performance work on large histories
- ⏳ Accessibility pass

## 6. v1.0 public launch ⏳

A stable release on macOS and Windows that anyone can download for free.

- ⏳ Public downloads on the website and GitHub Releases
- ⏳ User docs and FAQ
- ⏳ Security review of local storage
- ⏳ Release process and changelog

## 7. Encrypted sync ⏳

Optional sync between your Mac and Windows computers.

- ⏳ Choose the sync protocol and encryption scheme, reviewed in the open
- ⏳ End-to-end encryption, so clips are encrypted before they leave your device
- ⏳ Device pairing
- ⏳ Conflict handling and sync for pinboards
- ⏳ Decide whether the sync relay can be self-hosted

---

The roadmap will change as we learn. If you think something is missing or in the wrong order, open a discussion.
