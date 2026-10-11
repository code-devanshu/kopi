# Architecture (draft)

> [!NOTE]
> This is a draft. No desktop code exists yet, and everything here can change. Comments, corrections and alternatives are welcome: open a [discussion](https://github.com/code-devanshu/kopi/discussions) or a pull request against this file.

## Goals

- **Fast.** The window opens instantly from a hotkey, and search keeps up as you type, even with a large history.
- **Light.** Kopi runs in the background all day, so it should use little memory and almost no CPU when idle.
- **Private.** Clipboard data stays on the device unless the user turns on sync, and sync is end-to-end encrypted.
- **Native-feeling on both platforms.** It should follow macOS and Windows conventions rather than feel like a web page in a box.

## Planned stack

| Layer | Choice | Why |
| --- | --- | --- |
| App shell | [Tauri v2](https://v2.tauri.app) | Small binaries and low memory use compared with Electron, with one codebase for macOS and Windows |
| Core | Rust | Clipboard access, storage, search and OS integration, with direct access to native APIs |
| UI | React and TypeScript, built with Vite | Same skills as the website, so contributors can move between them |
| Storage | SQLite with the FTS5 extension | One local file, reliable, with full-text search built in |
| Large blobs | Files on disk, named by content hash | Keeps images out of the database so it stays small and fast |

Planned layout in the monorepo:

```text
apps/desktop/
├── src/             # React UI
└── src-tauri/       # Rust core and Tauri config
```

The UI talks to the core only through Tauri commands and events. The UI never reads the clipboard or the database directly.

## Data model

A first sketch. Names and fields will change.

### Clip

One item in the history.

| Field | Notes |
| --- | --- |
| `id` | Time-sortable ID (UUIDv7 or ULID) |
| `kind` | `text`, `link`, `image`, `color` or `code` |
| `text` | Plain text, used for display and search. Empty for images |
| `blob_path` | Path to the image or other binary data on disk, if any |
| `hash` | Content hash, used to spot duplicates |
| `source_app_id` | Bundle ID on macOS or executable path on Windows, if known |
| `source_app_name` | Display name of the source app |
| `size_bytes` | Size of the content |
| `created_at` | When it was first copied |
| `last_copied_at` | When it was last copied. Copying the same content again moves it to the top instead of making a duplicate |
| `copy_count` | How many times it has been copied or pasted |

A separate FTS5 table indexes `text` and `source_app_name` for search.

### Pinboard

A named collection of clips the user wants to keep.

| Field | Notes |
| --- | --- |
| `id` | |
| `name` | |
| `color` | Small visual marker |
| `position` | Order in the sidebar |
| `created_at` | |

A `pinboard_items` table links pinboards to clips, with a `position` for ordering. Pinned clips aren't removed by history limits.

### Settings

Stored as a versioned JSON file next to the database, so it's easy to inspect and migrate.

- Global hotkey, plus other shortcuts
- History limit, by item count, age or total size
- Ignored apps, never captured
- Which kinds to capture (for example, turn images off)
- Launch at login
- Theme
- Sync settings, once sync exists

## OS-specific pieces

Each of these lives behind a small Rust trait with one implementation per platform, so the rest of the core doesn't need to know which OS it's on.

| Piece | macOS | Windows |
| --- | --- | --- |
| **Clipboard watcher** | There's no change notification, so poll `NSPasteboard.changeCount` on a short timer | `AddClipboardFormatListener` and `WM_CLIPBOARDUPDATE` messages |
| **Global hotkey** | Tauri's global-shortcut plugin (Carbon hotkeys underneath) | Tauri's global-shortcut plugin (`RegisterHotKey` underneath) |
| **Paste into the active app** | Hide Kopi, return focus to the previous app, write the clip to the pasteboard and send ⌘V with `CGEvent`. Needs Accessibility permission | Same flow with `SetForegroundWindow` and `SendInput` for Ctrl+V. Can't paste into apps running as administrator |
| **Source app detection** | Frontmost app (`NSWorkspace`) when the change is seen, which is a best guess | Clipboard owner window (`GetClipboardOwner`), then its process, with the foreground window as a fallback |
| **Tray / menu bar icon** | Tauri tray API. Runs as an accessory app with no Dock icon | Tauri tray API, in the notification area |
| **Launch at login** | Tauri autostart plugin | Tauri autostart plugin |

## Privacy approach

- **Local-first.** History lives in the user's app data folder. Nothing goes over the network unless sync is turned on.
- **Skip sensitive copies.** On macOS, ignore pasteboard items marked with the [nspasteboard.org](http://nspasteboard.org) concealed or transient types, which most password managers set. On Windows, respect the `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory` formats.
- **User controls.** Ignore list for apps, a pause button, delete single items and clear all history.
- **Limits.** History limits are enforced on the device, and deleted items are removed from the search index too.
- **No telemetry by default.** The app sends no analytics. Any crash reporting would be opt-in.
- **Encrypted sync.** Clips are encrypted on the device with keys that never leave the user's devices. A sync server only sees ciphertext and the metadata it needs to deliver it.
- **Open code.** Anyone can check that the app does what this list says.

## Open questions

These are the decisions we'd most like help with. Comment in an issue or a discussion, or open a pull request that proposes an answer.

1. **macOS polling.** How often should we poll `changeCount` to feel instant without wasting battery? Should the rate adapt when the user is idle?
2. **Encryption at rest.** Should the local database be encrypted (for example with SQLCipher and a key in the Keychain or Windows Credential Manager), or is OS disk encryption enough?
3. **Rich formats.** Should we keep HTML and RTF alongside plain text so "paste with formatting" works? How much extra storage does that cost?
4. **Files.** Should copied files be supported, and if so, do we store paths or copies?
5. **Image limits.** What's a sensible default size limit for images, and should we generate thumbnails?
6. **Code detection.** How do we tell code from plain text reliably and cheaply?
7. **Sync design.** What should the protocol look like: a simple relay with an append-only encrypted log, or CRDTs? How do devices pair (for example, a QR code or a short code)? Should the relay be self-hostable?
8. **Core crate.** Should the platform-independent logic (storage, search, sync) be a separate Rust crate in `packages/`, so it can be tested without Tauri?
9. **Distribution.** Direct download only, or also the Mac App Store and Microsoft Store? Pasting into other apps needs Accessibility access, which doesn't fit well with the Mac App Store sandbox.
10. **Shared design tokens.** Should the website and the app share colors and type through a package in `packages/`?
11. **Other platforms.** Is Linux worth planning for in the architecture now, even if it ships later?
