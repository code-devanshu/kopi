# 0003. Watch the clipboard with our own native platform layer

- **Status:** Accepted. Revisited after the macOS pasteboard-privacy spike ([spike 0001](../spikes/0001-macos-pasteboard-access.md)).
- **Date:** 2026-10-11, revised 2026-10-12

## Context

Kopi has to notice every copy, read every useful format (text, RTF, HTML, images, file lists, colors), know which app the copy came from, and skip sensitive content without reading it.

- The official `tauri-plugin-clipboard-manager` (2.4.x) can read text and images and write HTML. It **can't watch for changes** and can't read RTF, file lists or custom types.
- macOS has no change notification. The standard approach is polling `NSPasteboard.changeCount`. Since macOS 15.4, programmatic reads of the general pasteboard can trigger a user alert, and macOS 27.2 appears to enforce that by default (see [PLATFORM_NOTES.md](../PLATFORM_NOTES.md#pasteboard-privacy-affects-the-whole-app)).
- Windows pushes `WM_CLIPBOARDUPDATE` to windows registered with `AddClipboardFormatListener`.

## Decision

Write a small platform layer (`crates/kopi-platform`) on the official bindings: **`objc2-app-kit`** for macOS and **`windows`** (windows-rs) for Windows. It sits behind a `Pasteboard` trait whose API forces a privacy-safe order:

1. `change_count()`: macOS polls it (250 ms while active, backing off to 500 ms when idle). Windows gets a push from a message-only listener window.
2. `types()`: format names only.
3. The privacy filter decides from types and source app: concealed, transient or auto-generated markers, Windows exclusion formats, excluded apps, paused.
4. `read(formats, max_bytes)`: only for allowed formats, up to size limits. The checks that need content (the size limit, and kinds such as link, code or color that are recognised from the text) run here, in memory. Content that fails them is dropped without being stored or logged.

On macOS, the layer also exposes `accessBehavior`, so the app can show a clear permission state. `read` refuses to run unless `accessBehavior` is `alwaysAllow`, and all pasteboard calls run on the main thread.

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
- **Bad:** more code to own, including unsafe FFI. Pasteboard calls must run on the main thread. On macOS 27.0.1, `accessBehavior` called from a background queue aborts the app.
- **Confirmed by the spike:** `changeCount` and `types` never prompt, so steps 1–3 work without any permission and concealed content is never read.
- **Risk:** once macOS enforces paste privacy (reportedly 27.2), Kopi needs **Always Allow** in System Settings → Privacy & Security. Clicking Allow in the alert covers only one copy, so without the setting Kopi would raise an alert for every copy. Onboarding must make the grant clear, and the app must detect and explain `.default`, `.ask` and `.alwaysDeny`.
- **Risk:** a content read without the grant blocks the main thread, which is also Tauri's event loop: about 60 s on the 26.6.2 preview, and with no time limit on the 27.0.1 preview. Gating reads on `alwaysAllow` should avoid this. If enforced 27.2 still blocks, move pasteboard reads into a helper process that can be killed, at the cost of a second binary to sign and a second grant.
- **Don't rely on the developer-preview flag** (`EnablePasteboardPrivacyDeveloperPreview`) for testing. On 27.0.1 it hangs pasteboard calls instead of showing the alert. Retest on real macOS 27.2 before building macOS onboarding.
