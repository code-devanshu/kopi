# Spike: paste into the active app

A small Tauri app for [issue #6](https://github.com/code-devanshu/kopi/issues/6). It reuses the non-activating panel from [spike 0002](../nspanel/README.md). Picking a clip writes it to the pasteboard, hides the panel and posts ⌘V with `CGEvent`. It lets us check:

- what happens with and without Accessibility
- which key code to use for V on each keyboard layout
- whether modifiers still held from the hotkey get in the way
- how long to wait between hiding and pasting
- whether it works in common apps

Results are written up in [docs/spikes/0003-macos-paste.md](../../docs/spikes/0003-macos-paste.md).

It uses Tauri 2.12, tauri-nspanel 2.1 and tauri-plugin-global-shortcut 2.4. It needs Rust ([rustup](https://rustup.rs)) and the Command Line Tools. It doesn't need the Tauri CLI or Node.

## Build and run

```sh
./build.sh --open    # build/KopiPasteSpike.app, bundle ID in.devanshuverma.kopi.pastespike, ad-hoc signed
```

The script runs `cargo build --release --features tauri/custom-protocol`, puts the binary and `Info.plist` (with `LSUIElement`) into an `.app` by hand, and signs it ad hoc. The first build takes a few minutes.

**Every build loses the Accessibility grant.** macOS ties the grant to the code signature, and each ad-hoc build gets a new one. Build once, grant Accessibility, and then change options from the menu bar icon instead of rebuilding. If a rebuilt app can't paste, remove the old KopiPasteSpike entry in System Settings → Privacy & Security → Accessibility and add it again.

## Using it

- **⇧⌘V** shows the panel over the app you were using. **↑/↓** choose a clip, **Return** pastes it, and **Esc** closes the panel.
- **⇧⌘1** pastes the first clip straight away, while ⇧ and ⌘ are still held. Use it to test held modifiers.
- The clips are plain text, two lines, Unicode, rich text (HTML), about 5,000 characters, and a shell command. Each one includes the paste number, so you can tell pastes apart.
- The **menu bar icon** has:
  - a status line showing whether Accessibility is granted (refreshed every second)
  - **Ask for Accessibility** (`AXIsProcessTrustedWithOptions` with the prompt), **Ask with CGRequestPostEventAccess**, and **Open Accessibility settings**
  - **V key code**: from the layout with ⌘ held (the default), from the layout with no modifiers, or always 9
  - **Delay after hiding**: 0 (the default), 10, 30, 50, 100 or 250 ms
  - **Keystrokes**: V down/up with the ⌘ flag (the default), or ⌘ down, V down/up, ⌘ up
  - **Event flags**: ⌘ plus the left-⌘ device bit (the default), ⌘ only, or whatever the event source sets
  - **Event source**: combined session state (the default), HID system state, or a private state
  - **Post to**: the session event tap (the default), the HID event tap, or the target app's pid
  - **Wait for modifiers to be released**, **Suppress local keys while posting** and **Hide on blur**
  - **Log the V key for every layout**, **Open log** and **Quit**

Settings reset on every launch.

## Log

Everything goes to stderr and to `~/Library/Logs/KopiPasteSpike.log`. At launch it logs the macOS version, the permission state, the current input source, and the V key code for every installed keyboard layout. Each paste logs:

- the clip, the target app and the pasteboard `changeCount`
- Accessibility, post-event access and secure input
- the input source, the layout and the key code used
- the settings
- the flags and characters on the V key-down event
- the time from hiding and from the trigger to posting
- the modifiers held just before posting, and whether Return was still down
- the frontmost app when posting, and whether it's the target
- 50 ms later, the focused app and the role of the focused element, from the Accessibility API

The log records labels and lengths, not the pasted text.

## Reset

```sh
pkill -x kopi-paste-spike
rm ~/Library/Logs/KopiPasteSpike.log
rm -rf build target gen
```

Then remove KopiPasteSpike from System Settings → Privacy & Security → Accessibility.
