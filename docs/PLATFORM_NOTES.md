# Platform notes

This is research for the desktop app, collected in October 2026. Versions and OS behaviour change, so re-check anything marked **verify** before relying on it. Sources are linked inline.

Contents:

- [Tauri v2](#tauri-v2)
- [macOS](#macos)
- [Windows](#windows)
- [Storage: SQLite and FTS5](#storage-sqlite-and-fts5)

---

## Tauri v2

### Versions

| Package | Latest stable | Source |
| --- | --- | --- |
| `tauri` (crate) | 2.12.3 (Oct 10, 2026) | [crates.io](https://crates.io/crates/tauri), [release notes](https://v2.tauri.app/release/) |
| `@tauri-apps/cli`, `@tauri-apps/api` | 2.12.0 (Sep 26, 2026) | [Tauri releases](https://v2.tauri.app/release/) |

Tauri 2.0 went stable on Oct 2, 2024. A 3.0 alpha exists, but 2.x is the stable line. Tauri's MSRV is Rust 1.90.

### Official plugins

| Plugin | Version | Fits? | Notes |
| --- | --- | --- | --- |
| [clipboard-manager](https://v2.tauri.app/plugin/clipboard/) | 2.4.2 | **No** (for capture) | Read/write plain text and images, write HTML, clear. **No change watching, no RTF, no file lists, no custom formats.** It's a thin wrapper over [`arboard`](https://github.com/1Password/arboard). Not enough for a clipboard manager. See [ADR 0003](decisions/0003-clipboard-watching-approach.md). |
| [global-shortcut](https://v2.tauri.app/plugin/global-shortcut/) | 2.4.1 | Yes | Built on [`global-hotkey`](https://github.com/tauri-apps/global-hotkey) 0.8. macOS uses Carbon `RegisterEventHotKey`, so no Input Monitoring or Accessibility permission is needed. Windows uses `RegisterHotKey`. The manager must live on the main thread on macOS. |
| [autostart](https://v2.tauri.app/plugin/autostart/) | 2.7.0 | Partly | Launch at login. On macOS we'd prefer `SMAppService` (a login item) over a LaunchAgent, because LaunchAgents trigger "Background items added" notifications. **Checked in 2.7.0:** the plugin has no `SMAppService` mode. It offers a LaunchAgent (the default, a plist in `~/Library/LaunchAgents`) or AppleScript through System Events (which needs Automation permission). The scaffold uses the LaunchAgent. It writes `~/Library/LaunchAgents/Kopi.plist` with the label `Kopi` (the app name, not the bundle ID), an empty `AssociatedBundleIdentifiers` and the absolute path to the binary, so moving the app breaks it. Settings ([#16](https://github.com/code-devanshu/kopi/issues/16)) should decide whether to call `SMAppService.mainApp` through `objc2` instead. |
| [sql](https://v2.tauri.app/plugin/sql/) | 2.5.0 | **No** | Exposes SQL to the frontend (sqlx-based). We keep the database in Rust (`rusqlite`), and the UI goes through typed commands only. |
| [updater](https://v2.tauri.app/plugin/updater/) | 2.13.3 | Later | Signed update manifests. Needed for beta, not the alpha. |
| [single-instance](https://v2.tauri.app/plugin/single-instance/) | 2.5.3 | Yes | A second launch focuses the running instance (or opens the panel). |
| [window-state](https://v2.tauri.app/plugin/window-state/) | 2.5.0 | Partly | Use it for the settings window only. The panel is positioned by our own code (cursor screen, centered). |
| [positioner](https://v2.tauri.app/plugin/positioner/) | 2.4.0 | Maybe | Helps place a window near the tray icon, if we add a tray-anchored mode. |

Versions are from [crates.io](https://crates.io) on Oct 11, 2026.

### Community options for the clipboard

| Option | Watches changes | Formats | Status | Verdict |
| --- | --- | --- | --- | --- |
| [`arboard`](https://github.com/1Password/arboard) 3.6.1 | No | Text, images, HTML (write) | Maintained by 1Password, widely used | Good reference, not enough on its own |
| [`clipboard-rs`](https://github.com/ChurchTao/clipboard-rs) 0.3.5 | Yes (macOS, Windows, Linux X11) | Text, RTF, HTML, PNG, file lists, custom types | Active (Jun 2026) | Closest fit, but the macOS watcher polls without exposing `changeCount`/types-before-read, and it has no privacy-marker handling |
| [`tauri-plugin-clipboard`](https://github.com/CrossCopy/tauri-plugin-clipboard) 2.1.11 | Yes | Text, RTF, HTML, image, files | Last release Oct 2024 | Stale, and it exposes the clipboard to JS. No. |
| [`clipboard-master`](https://github.com/DoumanAsh/clipboard-master) 4.0.0 | Yes (notification only) | None (no reads) | Small | Only a change signal |
| Native APIs through [`objc2-app-kit`](https://github.com/madsmtm/objc2) 0.3 and [`windows`](https://github.com/microsoft/windows-rs) 0.62 | We write it | Everything | Official bindings | **Chosen.** Gives full control over read order (types first, content only if allowed) |

### Tray / menu bar app

- Enable the `tray-icon` Cargo feature. Build the icon with `TrayIconBuilder` in `setup`, attach a `Menu`, and handle `on_menu_event` and `on_tray_icon_event` ([docs](https://v2.tauri.app/learn/system-tray/)). Use `.show_menu_on_left_click(false)` if left-click should open the panel instead.
- On macOS, use a template image for the menu bar icon, so it adapts to light and dark menu bars.

### Panel window

What we need: borderless, always on top, appears on a global shortcut, hides on blur, no Dock icon, and it shows over full-screen apps. On macOS this was verified in [spike 0002](spikes/0002-macos-nspanel.md) on macOS 27.0.1.

| Need | How |
| --- | --- |
| Borderless, on top, hidden by default | Window config: `decorations: false`, `skipTaskbar: true`, `visible: false`, `resizable: false`. For rounded corners, use `transparent: true` plus `macOSPrivateApi: true` (and the `macos-private-api` crate feature), and set `border-radius` in CSS. |
| Show on shortcut (macOS) | `global-shortcut` handler: center the panel on the visible frame of the screen with the cursor, then call tauri-nspanel's `show_and_make_key()`. **Don't use Tauri's `set_focus()`**: tao calls `activateIgnoringOtherApps`, which activates Kopi. Build the panel once at launch, hidden. Showing it then takes under 10 ms. |
| Hide on blur | Handle Tauri's `WindowEvent::Focused(false)`, then hide. Wait about 100 ms and skip the hide if the Character Palette (⌃⌘Space) or an input method took key status. Otherwise, emoji input from the panel is lost. Don't use tauri-nspanel's `set_event_handler` for this, because it replaces tao's delegate. |
| No Dock icon (macOS) | **Both** `LSUIElement = true` in Info.plist **and** `app.set_activation_policy(ActivationPolicy::Accessory)` in `setup`. With only the runtime call, the Dock icon flashes at launch. With only `LSUIElement`, the policy still reads Regular once `setup` runs. |
| Over full-screen apps, without stealing activation (macOS) | Convert the window to an `NSPanel` with [`tauri-nspanel`](https://github.com/ahkohd/tauri-nspanel) 2.1: `nonactivating_panel` style, `Floating` level (4), `can_join_all_spaces` + `full_screen_auxiliary`, `hides_on_deactivate(false)`, `no_activate(true)`. The target app keeps the menu bar and stays frontmost while the panel has keyboard focus, which also simplifies paste. Background: [Multi's write-up on Spotlight-style panels](https://multi.app/blog/nailing-the-activation-behavior-of-a-spotlight-raycast-like-command-palette). |
| "Is Kopi in front?" (macOS) | Use `NSWorkspace.frontmostApplication`. `NSApp.isActive` reads true whenever the panel is key, even though Kopi isn't frontmost. |
| Keys in the webview | Skip Kopi's shortcuts when `e.isComposing \|\| e.keyCode === 229`. The press-and-hold accent picker sends `keyCode` 229 with `isComposing` false and no composition events. `document.hasFocus()` turns true one frame after the panel is shown. |
| Windows | A normal Tauri window works. Bring it forward with `set_focus()`, and remember the previous foreground `HWND` before showing it. |

---

## macOS

### Pasteboard privacy (affects the whole app)

Apple has added a permission model for reading the general pasteboard:

- **API (macOS 15.4+):** `NSPasteboard.accessBehavior` returns `.default`, `.ask`, `.alwaysAllow` or `.alwaysDeny` ([Apple docs](https://developer.apple.com/documentation/appkit/nspasteboard/accessbehavior-swift.enum)). Per Apple, the general pasteboard's default "is to ask upon programmatic access". The first alert moves the app to `.ask`, and from then on the user can switch it between Ask, Allow and Deny in System Settings. Access that's "user originated and paste related" (⌘V, a Paste button) is always allowed.
- **Reading without an alert:** `detectedPatterns(for:)` and `detectedMetadata(for:)` inspect the first item "without notifying the person using the app" ([Apple docs](https://developer.apple.com/documentation/appkit/nspasteboard)).
- **History:** it was a developer preview in macOS 15.4 (`defaults write <bundle id> EnablePasteboardPrivacyDeveloperPreview -bool yes`). It reportedly wasn't enforced in macOS 26 ([Michael Tsai](https://mjtsai.com/blog/2025/05/12/pasteboard-privacy-preview-in-macos-15-4/)). The **macOS 27.2 beta 2 release notes remove that developer-preview default** ([summary of the notes](https://betaprofile.dev/macos-27-2-beta-2/), [second summary](https://blakecrosley.com/blog/macos-27-release-notes)). Users report that apps reading the clipboard are now restricted by default ([Parallels forum, Oct 2026](https://forum.parallels.com/threads/problem-with-clipboard-macos-27.371175/)). This isn't confirmed by Apple or on real hardware yet ([open probe](https://github.com/admax12959/OneShot/issues/15)).
- **Spike results** ([spike 0001](spikes/0001-macos-pasteboard-access.md), macOS 26.6.2 and 27.0.1):
  - `changeCount`, `types` and `pasteboardItems.count` never trigger the alert. The first `string`/`data` read of each copy does.
  - Clicking **Allow Paste** in the alert covers only that one copy, and the app stays in `.ask`. It is not a standing grant.
  - The standing grant lives in System Settings → Privacy & Security, with the options Ask, Always Allow and Deny. Always Allow sets `.alwaysAllow`, and it survived an OS upgrade and a reboot.
  - On 26.6.2 an unanswered read blocks for about 60 s, then returns `nil`.
  - **Nothing is enforced on 27.0.1.** Without the developer-preview flag, `accessBehavior` is `.alwaysAllow` and reads work even when System Settings says Ask.
  - **With the flag on, 27.0.1 hangs instead of alerting.** The Trusted UI service that shows the alert isn't registered, so pasteboard reads (and sometimes `accessBehavior`) block the main thread with no time limit. Don't use the flag on 27.0.1.
  - **Pasteboard calls must stay on the main thread.** On 27.0.1, `accessBehavior` called from a background queue raises `NSInternalInconsistencyException` and the app aborts.

**What this means for Kopi:**

1. Treat "Paste from Other Apps: Allow" as a **required permission** on macOS 27.2+, like Accessibility. Onboarding explains why and sends the user to System Settings → Privacy & Security. Apple asks apps to guide users there through onboarding rather than deep-linking from the alert.
2. Check `accessBehavior` at launch and whenever the panel opens. Show a clear "Kopi can't see your clipboard" state for `.default`, `.ask` and `.alwaysDeny`, instead of failing silently.
3. Read in this order: `changeCount`, then `types`, then the privacy filter, and only then the content. The first three never prompt, so Kopi can detect copies and skip concealed content without an alert.
4. Read content only when `accessBehavior` is `.alwaysAllow`. A read without the grant blocks the main thread, which is also Tauri's event loop, and it can't be moved to a background thread.
5. Still open: enforcement on real macOS 27.2, and whether the grant survives an app update (TCC is tied to the code signature). Test both before building macOS onboarding.

### Detecting clipboard changes

- There's no change notification. Poll `NSPasteboard.general.changeCount`, which increments on each ownership change ([Apple docs](https://developer.apple.com/documentation/appkit/nspasteboard/changecount)).
- Maccy polls every 0.5 s by default (`clipboardCheckInterval`, [source](https://github.com/p0deje/Maccy/blob/master/Maccy/Extensions/Defaults.Keys%2BNames.swift)). Kopi plans 250 ms while the user is active and backs off to 500 ms when idle. Reading an integer is cheap, but timers prevent App Nap, so measure energy impact in Activity Monitor.
- Copies made within one polling interval collapse into one change. That's acceptable.

### Reading types

| Content | Type (UTI) |
| --- | --- |
| Plain text | `public.utf8-plain-text` (`NSPasteboard.PasteboardType.string`) |
| Rich text | `public.rtf`, `com.apple.flat-rtfd` |
| HTML | `public.html` |
| Images | `public.png`, `public.tiff` (screenshots and many apps provide TIFF) |
| Files | `public.file-url`, one `NSPasteboardItem` per file |
| URL | `public.url` |
| Color | `com.apple.cocoa.pasteboard.color` (an archived `NSColor`, read with `NSColor(from:)`) |

Read each item from `pasteboardItems`, not just the first item, because multiple files or mixed content use several items.

### Detecting the source app

- `NSWorkspace.shared.frontmostApplication` at the moment the change is seen ([Apple docs](https://developer.apple.com/documentation/appkit/nsworkspace/frontmostapplication)). It's a best guess: a background app, a script or a menu bar app can write the pasteboard while another app is frontmost.
- Prefer `org.nspasteboard.source` if it's present. Some apps write their own bundle ID there ([nspasteboard.org](http://nspasteboard.org/)). Microsoft apps write `com.microsoft.ole.source.*` types, which Maccy also checks.

### Skipping sensitive content

The [nspasteboard.org](http://nspasteboard.org/) conventions, which most password managers follow:

| Marker type | Meaning | Kopi |
| --- | --- | --- |
| `org.nspasteboard.ConcealedType` | Confidential, such as a password | **Never read or store** |
| `org.nspasteboard.TransientType` | On the pasteboard only briefly | **Never read or store** |
| `org.nspasteboard.AutoGeneratedType` | Not from a user Copy action | Don't store |
| `org.nspasteboard.source` | Bundle ID of the originating app | Use as the source app |

Legacy markers to honour too: `com.agilebits.onepassword`, `de.petermaurer.TransientPasteboardType`, `com.typeit4me.clipping`, `Pasteboard generator type`, `net.antelle.keeweb` (Maccy's default ignore list, [source](https://github.com/p0deje/Maccy/blob/master/Maccy/Extensions/Defaults.Keys%2BNames.swift)).

These checks use only `types`, before any content is read.

### Pasting into the active app

Verified in [spike 0003](spikes/0003-macos-paste.md) on macOS 27.0.1, in Notes, TextEdit, Safari, Chrome, VS Code and Terminal.

1. Check `AXIsProcessTrusted()`. If it's false, stop here: leave the clip on the pasteboard and show "Copied. Press ⌘V".
2. On the main thread, find the key code for V in the current layout (see Keyboard layouts below).
3. Write the clip's formats to the pasteboard, with `org.nspasteboard.source` set to Kopi's bundle ID so the watcher ignores it. Plain text plus `public.html` kept the formatting in TextEdit and Notes.
4. Hide the panel. With the non-activating panel, the target app is still frontmost and key. Otherwise, activate the remembered `NSRunningApplication`.
5. Right away, with no delay, post V down and V up with `CGEventCreateKeyboardEvent`:
   - a `combinedSessionState` event source
   - flags set explicitly to `kCGEventFlagMaskCommand | NX_DEVICELCMDKEYMASK` (`0x100008`)
   - posted to `kCGSessionEventTap`

   In the spike, this posted 0.5–1.4 ms after hiding, and no app missed it.

Gotchas:

- **Permission:** posting events into other apps needs **Accessibility** (System Settings → Privacy & Security → Accessibility). Without it, `CGEventPost` **drops the events silently**, so check `AXIsProcessTrusted()` first. Ask with `AXIsProcessTrustedWithOptions` and `kAXTrustedCheckOptionPrompt` ([Apple docs](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions)). That call returns false at once and shows the alert, so poll `AXIsProcessTrusted()` afterwards. The grant takes effect without a relaunch. **Don't use `CGPreflightPostEventAccess`**: it stayed false while Accessibility was granted and pastes worked.
- **Keyboard layouts:** the virtual key code for V differs between layouts. Run `UCKeyTranslate` on `TISCopyCurrentKeyboardLayoutInputSource()` with ⌘ held (modifier state 1), scan key codes 0–127 for "v", and fall back to 9. Of 264 layouts, that gives 9 on Latin and non-Latin layouts and on "Dvorak – QWERTY ⌘", 47 on Dvorak, 43 on Dvorak – Right-Handed and 8 on Turkish F. Without ⌘ held, "Dvorak – QWERTY ⌘" gives the wrong key and 122 non-Latin layouts give none ([Maccy #482](https://github.com/p0deje/Maccy/issues/482)). Text Input Sources must be read on the main thread.
- **Held modifiers:** the user may still be holding ⇧⌘ from the hotkey, and Return is usually still down. `CGEventSetFlags` replaces the event's flags, so the held ⇧ doesn't reach the app. No need to wait for release.
- **No success signal:** nothing reports whether the paste landed. An Accessibility query right after a rich-text paste can time out (−25204) while the app is busy, so don't use it to verify.
- **Dev builds:** TCC grants (Accessibility, pasteboard) are tied to the code signature. Ad-hoc-signed builds lose them on every rebuild, so sign dev builds with a stable identity.
- **Sandbox:** posting events and background pasteboard reads don't fit the App Sandbox. Kopi ships outside the Mac App Store.

### Distribution

- Use Developer ID signing plus notarization (Apple Developer Program). Since macOS 15 Sequoia, users can no longer Control-click to open un-notarized apps. They have to approve them in System Settings → Privacy & Security ([The Hacker News](https://thehackernews.com/2024/08/apples-new-macos-sequoia-tightens.html)). That's acceptable for a small alpha, not for 1.0.
- Tauri's bundler supports signing and notarization through environment variables in CI ([Tauri docs](https://v2.tauri.app/distribute/sign/macos/)).

### macOS's own clipboard history

macOS 26 Tahoe added clipboard history in Spotlight. It's off by default ([ClipBook guide](https://clipbook.app/blog/how-to-see-clipboard-history-on-mac/)). It doesn't conflict with Kopi, but it sets a baseline: Kopi has to be clearly better (search, pinboards, privacy, Windows).

---

## Windows

### Detecting clipboard changes

- Use `AddClipboardFormatListener(hwnd)`. The window receives `WM_CLIPBOARDUPDATE` on every change. This is Microsoft's recommended method. Clipboard viewer chains are legacy, and `GetClipboardSequenceNumber` "should not be used in a polling loop" ([Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/dataxchg/using-the-clipboard)).
- Create a message-only window (`HWND_MESSAGE`) on a dedicated thread with its own message loop for the listener. Keep the handler short and send the work to the core.
- `OpenClipboard` fails while another app holds the clipboard. Retry a few times with a short backoff (for example 5 × 20 ms), then give up for this change.

### Clipboard formats

| Content | Format |
| --- | --- |
| Text | `CF_UNICODETEXT` (the system synthesises `CF_TEXT`/`CF_OEMTEXT`) |
| Images | `PNG` (registered, used by browsers and Office), `CF_DIBV5` / `CF_DIB` |
| Files | `CF_HDROP` (`DragQueryFileW`) |
| Rich text | `Rich Text Format` (registered) |
| HTML | `HTML Format` (registered, with a header giving fragment offsets) |
| URL | `UniformResourceLocatorW` |

Registered formats are looked up with `RegisterClipboardFormatW`, and their numeric IDs differ between sessions. Prefer `CF_DIBV5` over `CF_BITMAP`, because the system converts it to sRGB ([Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats)).

### Skipping sensitive content

Password managers mark their copies with formats from Microsoft's [Cloud Clipboard and Clipboard History Formats](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#cloud-clipboard-and-clipboard-history-formats):

| Format | Meaning | Kopi |
| --- | --- | --- |
| `ExcludeClipboardContentFromMonitorProcessing` | Any data in this format: exclude from history and cloud sync | **Never read or store** |
| `CanIncludeInClipboardHistory` | `DWORD` 0 = keep out of history, 1 = explicitly allowed | **Skip when 0** |
| `CanUploadToCloudClipboard` | `DWORD` 0 = don't sync to other devices | Store locally. Respect it when sync exists. |

Check these with `IsClipboardFormatAvailable` before reading any content.

### Detecting the source app

- `GetClipboardOwner()` returns the window that last emptied the clipboard. It can be `NULL`, or a hidden helper window, for example with delayed rendering or when the owner has exited.
- From the `HWND`, use `GetWindowThreadProcessId`, then `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` and `QueryFullProcessImageNameW` to get the exe path. Use the file's version info (`FileDescription`) for the display name. For packaged (UWP/MSIX) apps, use `GetApplicationUserModelId`.
- Fall back to `GetForegroundWindow()` when the owner is unknown.

### Pasting into the active app

1. Before showing Kopi, record `GetForegroundWindow()`.
2. Write the clip to the clipboard (Kopi's window becomes the owner, which is how the watcher recognises its own writes).
3. Hide Kopi, then `SetForegroundWindow(previous)`. Kopi is the foreground process at that moment, so the foreground-lock rules allow it.
4. `SendInput` Ctrl down, V down, V up, Ctrl up. Check `GetAsyncKeyState` and release any modifiers the user is still holding.

Gotchas:

- **UIPI:** "Applications are permitted to inject input only into applications that are at an equal or lesser integrity level", and the failure isn't reported ([Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)). Kopi can't paste into apps running as administrator. Don't run Kopi elevated. Detect the target's integrity level and fall back to "copied, press Ctrl+V".
- Some apps (terminals, games) ignore or remap Ctrl+V. A per-app "paste with Shift+Insert" option can come later.
- No special permission is needed.

### Code signing and SmartScreen

- Since 2024, **EV certificates no longer bypass SmartScreen**. EV and OV both build reputation over time ([Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options), [Tauri docs](https://v2.tauri.app/distribute/sign/windows/)).
- **Azure Artifact Signing** (formerly Trusted Signing) is Microsoft's low-cost option. It needs a paid Azure subscription and doesn't give instant reputation. Individual developers must be in the **US or Canada**, and organisations in a listed set of countries ([quickstart prerequisites](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart), [FAQ](https://learn.microsoft.com/en-us/azure/artifact-signing/faq)).
- **SignPath Foundation** offers free signing for open-source projects that build in CI, with keys held in their HSM ([signpath.org](https://signpath.org/)). This is a likely fit for Kopi. **Verify** the eligibility terms.
- Unsigned builds show "Windows protected your PC". That's tolerable for testers, not for 1.0.

### Windows's own clipboard history

Win+V clipboard history and cloud clipboard coexist with Kopi. Both honour the same exclusion formats.

---

## Storage: SQLite and FTS5

- `rusqlite` 0.40 with the `bundled` feature compiles SQLite 3.54 with `SQLITE_ENABLE_FTS5` ([build script](https://github.com/rusqlite/rusqlite/blob/master/libsqlite3-sys/build.rs)). We get the same SQLite version on every OS, regardless of the system library.
- **External content tables** keep text stored once. Triggers must keep the index in sync, and `'rebuild'` repairs it ([FTS5 docs](https://www.sqlite.org/fts5.html#external_content_tables)).
- The **trigram tokenizer** supports substring search, including `LIKE`/`GLOB` acceleration. Queries shorter than 3 characters match nothing, so we need a fallback ([FTS5 docs](https://www.sqlite.org/fts5.html#the_trigram_tokenizer)).
- The **`secure-delete` option** (3.42+) removes deleted terms from the index instead of leaving them until a merge. This matters for privacy ([FTS5 docs](https://www.sqlite.org/fts5.html)).
- **Blobs:** for objects larger than about 100 KB, separate files read faster than in-database BLOBs. Use 8–16 KiB pages ([SQLite: Internal vs External BLOBs](https://www.sqlite.org/intern-v-extern-blob.html)). Kopi stores up to 64 KiB inline, and larger data as content-addressed files.

Details are in [DATA_MODEL.md](DATA_MODEL.md).
