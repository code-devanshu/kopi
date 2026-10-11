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

- 🔨 Draft the architecture ([docs/ARCHITECTURE.md](docs/ARCHITECTURE.md))
- ⏳ Settle the open questions in that doc with contributors
- ⏳ Design the main window: history list, search, filters and preview
- ⏳ Design pinboards, settings and onboarding
- ⏳ Define the keyboard shortcuts
- ⏳ Spike the risky parts: clipboard watching, global hotkey and pasting into the active app on both platforms

## 3. macOS alpha ⏳

First usable builds for early testers on macOS.

- ⏳ Tauri app scaffold in `apps/desktop`
- ⏳ Clipboard watcher for text, links, images and colors
- ⏳ Local history stored in SQLite
- ⏳ Full-text search with type and source-app filters
- ⏳ Global hotkey, keyboard navigation and paste into the active app
- ⏳ Skip copies from password managers and content marked as concealed
- ⏳ Menu bar icon and launch at login

## 4. Windows alpha ⏳

The same app, built for Windows.

- ⏳ Windows clipboard watcher, hotkey and paste
- ⏳ Source-app detection on Windows
- ⏳ Tray icon and launch at login
- ⏳ Feature parity with the macOS alpha

## 5. Private beta ⏳

A wider group of testers on both platforms, with the features rounded out.

- ⏳ Pinboards
- ⏳ Paste as plain text
- ⏳ Settings: history limits, ignored apps and shortcuts
- ⏳ Signed builds and automatic updates
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
