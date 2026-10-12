# Spike: non-activating panel

A small Tauri app for [issue #5](https://github.com/code-devanshu/kopi/issues/5). ⇧⌘V shows a borderless panel on the screen with the cursor, built as a non-activating `NSPanel` with [`tauri-nspanel`](https://github.com/ahkohd/tauri-nspanel). It lets us check whether the panel gets keyboard focus without activating Kopi, whether it works over full-screen apps, and whether a Dock icon shows. Results are written up in [docs/spikes/0002-macos-nspanel.md](../../docs/spikes/0002-macos-nspanel.md).

It uses Tauri 2.12, tauri-nspanel 2.1 and tauri-plugin-global-shortcut 2.4. It needs Rust ([rustup](https://rustup.rs)) and the Command Line Tools. It doesn't need the Tauri CLI or Node.

## Build and run

```sh
./build.sh --open           # build/KopiPanelSpike.app, bundle ID in.devanshuverma.kopi.panelspike, ad-hoc signed
LSUI=1 ./build.sh --open    # same, with LSUIElement = true in Info.plist
```

The script runs `cargo build --release --features tauri/custom-protocol`, which embeds `ui/` in the binary the way `tauri build` would. It then puts the binary and `Info.plist` into an `.app` by hand and signs it ad hoc. The first build takes a few minutes.

The frontend lives in `ui/`, not `dist/`, because the repo's root `.gitignore` ignores `dist/`.

## Using it

- **⇧⌘V** shows or hides the panel.
- In the panel, **Esc** hides it, **↑/↓** move the selection, and **Return** "picks" a clip and hides the panel. Clicking anywhere else hides it too.
- The **menu bar icon** has:
  - **Show panel**
  - **Focus method**: tauri-nspanel's `show_and_make_key` (the default), the same plus making the `WKWebView` first responder, or Tauri's own `show()` + `set_focus()`
  - **Panel level**: Floating (4, the default), Status (25) or Pop-up menu (101)
  - **Hide on blur**
  - **Open log** and **Quit**

Settings reset on every launch.

## Log

Everything goes to stderr and to `~/Library/Logs/KopiPanelSpike.log`. At launch it logs the macOS version, the activation policy and the panel's class, level, style mask and collection behaviour. Each show and hide logs:

- the frontmost app before and after
- whether Kopi is active
- whether the panel is key
- the first responder
- the screen
- timings

The page logs its focus state, key names, key codes and composition events. It never logs typed characters, only key names and lengths.

## Reset

```sh
pkill -x kopi-panel-spike
rm ~/Library/Logs/KopiPanelSpike.log
rm -rf build target gen
```
