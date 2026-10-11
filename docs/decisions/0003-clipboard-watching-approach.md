# 0003. Watch the clipboard with our own native platform layer

- **Status:** Accepted, pending the macOS pasteboard-privacy spike
- **Date:** 2026-10-11

## Context

Kopi has to notice every copy, read every useful format (text, RTF, HTML, images, file lists, colors), know which app the copy came from, and skip sensitive content without reading it.

- The official `tauri-plugin-clipboard-manager` (2.4.x) can read text and images and write HTML. It **can't watch for changes** and can't read RTF, file lists or custom types.
- macOS has no change notification. The standard approach is polling `NSPasteboard.changeCount`. Since macOS 15.4, programmatic reads of the general pasteboard can trigger a user alert, and macOS 27.2 appears to enforce that by default (see [PLATFORM_NOTES.md](../PLATFORM_NOTES.md#pasteboard-privacy-affects-the-whole-app)).
- Windows pushes `WM_CLIPBOARDUPDATE` to windows registered with `AddClipboardFormatListener`.

## Decision

Write a small platform layer (`crates/kopi-platform`) on the official bindings: **`objc2-app-kit`** for macOS and **`windows`** (windows-rs) for Windows. It sits behind a `Pasteboard` trait whose API forces a privacy-safe order:

1. `change_count()`: macOS polls it (250 ms while active, backing off when idle). Windows gets a push from a message-only listener window.
2. `types()`: format names only.
3. The privacy filter decides from types and source app: concealed, transient or auto-generated markers, Windows exclusion formats, excluded apps, paused.
4. `read(formats, max_bytes)`: only for allowed formats, up to size limits.

On macOS, the layer also exposes `accessBehavior`, so the app can show a clear permission state.

## Alternatives considered

| Option | Why not |
| --- | --- |
| `tauri-plugin-clipboard-manager` | No watching, and too few formats. |
| `clipboard-rs` | Watches on all platforms and reads most formats, so it's the closest fit. But its watcher reads on every change and doesn't let us inspect types and apply privacy rules before content is touched. We'd also depend on one maintainer for a core path. We may borrow from its implementation (MIT/Apache). |
| `tauri-plugin-clipboard` (CrossCopy) | Last release Oct 2024, and it exposes clipboard data to JavaScript. |
| `arboard` + `clipboard-master` | No change watching on its own (arboard), and no reading at all (clipboard-master). Two crates that still miss RTF, files and markers. |
| Swift / C# helper process | Native-language comfort, but adds IPC, a second binary to sign, and a second language. `objc2`/`windows-rs` reach the same APIs from Rust. |

## Consequences

- **Good:** full control over what is read and when, and it honours every privacy marker. One code path per OS, unit-testable behind traits.
- **Bad:** more code to own, including unsafe FFI. The macOS pasteboard API has threading caveats that the spike must check (reads off the main thread).
- **Risk:** if macOS prompts on `changeCount` or `types` without the Allow permission, Kopi can't work at all until the user grants it. Onboarding must make that grant clear, and the app must detect and explain `.ask` and `.alwaysDeny`. This ADR is revisited after the spike.
