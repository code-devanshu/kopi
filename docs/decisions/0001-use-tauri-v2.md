# 0001. Use Tauri v2 for the desktop app

- **Status:** Accepted
- **Date:** 2026-10-11

## Context

Kopi runs in the background all day on macOS and Windows. It must open instantly, use little memory, and call low-level OS APIs: clipboard, input injection, window activation and accessibility. The website is built with React and TypeScript, and we want contributors to be able to move between the two.

## Decision

Build the desktop app with **Tauri v2** (stable line, 2.12.x as of Oct 2026). The core is Rust and the UI is React and TypeScript in the system webview (WKWebView on macOS, WebView2 on Windows).

- Platform-independent logic goes in Rust crates that don't depend on Tauri (`crates/kopi-core`, `crates/kopi-platform`). The Tauri app is a thin shell.
- Official plugins used: `global-shortcut`, `autostart`, `single-instance`, `window-state`, `log`, and later `updater`. We don't use `clipboard-manager` or `sql` (see ADRs 0002 and 0003).
- On macOS, the panel window becomes an `NSPanel` through `tauri-nspanel`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| **Electron** | 100+ MB installs and high idle memory for an always-running utility. Native clipboard and input work would still need native modules. |
| **Fully native** (Swift/AppKit and C#/WinUI) | The best platform feel, but two separate codebases and UI stacks for a small volunteer team, and nothing shared for Windows. |
| **Flutter** | Desktop clipboard and input plugins are thin, so we'd still write native code. It's also a third language for contributors. |
| **Rust GUI** (egui, Iced, Slint) | Smallest footprint, but immature text input, accessibility and native feel compared with a webview, and fewer contributors know them. |

## Consequences

- **Good:** small binaries and low memory, one UI codebase, and a Rust core with direct access to native APIs through `objc2` and `windows-rs`. The web stack is shared with the website.
- **Bad:** webview rendering differs slightly between WKWebView and WebView2, so the UI is tested on both. Some native behaviour (non-activating panels, full-screen spaces) needs community plugins or our own `objc2` code.
- **Risk:** the panel's activation behaviour on macOS is the riskiest part of the UI. It gets a spike before the UI is built on it.
