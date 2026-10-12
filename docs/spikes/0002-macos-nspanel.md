# Spike 0002: non-activating panel on macOS

- **Issue:** [#5](https://github.com/code-devanshu/kopi/issues/5)
- **Date:** 2026-10-13
- **Code:** [spikes/nspanel](../../spikes/nspanel/README.md)
- **Feeds into:** [ADR 0001](../decisions/0001-use-tauri-v2.md), [ADR 0004](../decisions/0004-paste-simulation-approach.md), [PLATFORM_NOTES.md](../PLATFORM_NOTES.md#panel-window)

## Question

Can a Tauri window, turned into an `NSPanel` with `tauri-nspanel`, behave like Kopi's panel? That means:

- borderless, on the screen with the cursor, over full-screen apps
- no Dock icon
- keyboard focus without activating Kopi
- hides on blur and on Esc

What do we have to do to get there, and what goes wrong?

## Setup

| | |
| --- | --- |
| macOS | 27.0.1 (26A434), one display |
| Stack | Tauri 2.12.3, tauri-nspanel 2.1.1, tauri-plugin-global-shortcut 2.4.1, Rust 1.99.0 |
| App | `in.devanshuverma.kopi.panelspike` 0.1.0 (1), release build, ad-hoc signed, not sandboxed. Built without the Tauri CLI (see the README). |
| Hotkey | ⇧⌘V, through tauri-plugin-global-shortcut (Carbon `RegisterEventHotKey`, no permission needed) |

The panel is built once in `setup` and then shown and hidden:

```rust
app.set_activation_policy(ActivationPolicy::Accessory);

PanelBuilder::<_, KopiPanel>::new(app.handle(), "panel")
    .level(PanelLevel::Floating)                                   // 4
    .style_mask(StyleMask::empty().nonactivating_panel())
    .collection_behavior(CollectionBehavior::new().can_join_all_spaces().full_screen_auxiliary())
    .hides_on_deactivate(false)
    .transparent(true)
    .no_activate(true)
    .with_window(|w| w.decorations(false).transparent(true).visible(false).skip_taskbar(true))
    .build()?;
```

`KopiPanel` is declared with `tauri_panel!`, with `can_become_key_window: true`, `can_become_main_window: false` and `is_floating_panel: true`.

The resulting panel reads:

- level 4
- styleMask `0x80` (non-activating only, no title bar)
- collectionBehavior `0x101` (`canJoinAllSpaces` + `fullScreenAuxiliary`)
- `hidesOnDeactivate` false

To show the panel:

1. Center it on the visible frame of the screen with the cursor.
2. Call tauri-nspanel's `show_and_make_key()`. That's `makeFirstResponder(contentView)`, then `orderFrontRegardless`, then `makeKeyWindow`.
3. Emit `panel-shown` to the page, which focuses the search input.

To hide it, call `panel.hide()` on Esc, on Return or on Tauri's `WindowEvent::Focused(false)`.

The target app in every test was VS Code. The full-screen test used Chrome in full screen.

## Results

### Activation and focus

| Check | Result |
| --- | --- |
| Menu bar while the panel is open | **Stayed on Code.** Kopi's menu never appeared. |
| `NSWorkspace.frontmostApplication` while open and after hiding | **Code**, every time. After Esc or Return, "same app as before show" was true. |
| `NSApp.isActive` while the panel is key | **true**, even though Kopi isn't frontmost. It returns to false once the panel hides. |
| Typing into the panel right after ⇧⌘V, with no click | **Works.** The panel is key, and `WryWebView` is already first responder after `show_and_make_key`. |
| Typing after Esc | **Goes to Code**, with no click needed. |
| `document.hasFocus()` in the `panel-shown` handler | **false**, both before and right after `input.focus()`. **true** on the next animation frame. `document.activeElement` is the input the whole time. |
| ⌘Tab | Kopi isn't in the app switcher. |

### Show and hide

| Check | Result |
| --- | --- |
| Esc | Hides the panel. |
| Click in another window | `Focused(false)` arrives, and the panel hides. |
| Over full-screen Chrome (level 4) | **The panel appears over the full-screen app, and the user stays in that space.** |
| Native show call, `show_and_make_key` | 3.5–8.6 ms |
| Hotkey to panel shown (native) | 4.4–9.2 ms |
| Hotkey to `window focus` in JS | 150 ms on the first show, then 6–14 ms |
| Hotkey to the `panel-shown` handler in JS | 154 ms on the first show, then 19–60 ms |
| Building the panel at launch | 770 ms on the first launch after a build, 95–205 ms on later launches |

### Text input

| Check | Result |
| --- | --- |
| Press-and-hold accent picker (hold E) | The picker appears **above the panel**, and choosing é inserts it into the input. While the key is held, `keydown` repeats with **`keyCode` 229 and `isComposing` false**. No `composition*` events fire at all. |
| Emoji & Symbols (⌃⌘Space), hide on blur **on** | **The panel hides and the emoji is lost.** The Character Palette takes key status about 250 ms after ⌃⌘Space, which sends `Focused(false)`. |
| Emoji & Symbols, hide on blur **off** | **The emoji is inserted.** `Focused(false)` arrives when the palette opens and `Focused(true)` when it closes. Code stayed frontmost throughout. |

### Dock icon

The app was quit before each launch, and the user watched the Dock.

| Build | Dock icon at launch? | `NSApp.isActive` right after building the panel |
| --- | --- | --- |
| `set_activation_policy(Accessory)` in `setup` only | **Yes, a brief flash**, then gone | true |
| Same, plus `LSUIElement = true` in Info.plist | **No**, over two launches | false |

In both builds, `activationPolicy` reads **Regular** when `setup` starts, even with `LSUIElement`. Something in the launch path, most likely tao, sets it before our code runs. So `LSUIElement` alone isn't enough, and Kopi still has to call `set_activation_policy(Accessory)`.

## Answers

| Question | Answer |
| --- | --- |
| Borderless panel on the screen with the cursor? | **Yes.** It's positioned by our own code from `NSEvent.mouseLocation` and `NSScreen.visibleFrame`. Tested on one display only. |
| Over full-screen apps? | **Yes**, at level 4 with `canJoinAllSpaces` + `fullScreenAuxiliary`. |
| No Dock icon? | **Yes**, with `LSUIElement` in Info.plist **and** `Accessory` at runtime. With `Accessory` alone, the icon flashes at launch. |
| Not in ⌘Tab? | **Yes.** |
| Keyboard focus without activating Kopi? | **Yes.** The target app keeps the menu bar and stays frontmost, and keys go to the panel without a click. |
| Hides on Esc? | **Yes.** It's handled in JS and calls a `hide` command. |
| Hides on blur? | **Yes**, through `WindowEvent::Focused(false)`. But it needs an exception for the emoji picker (see below). |
| IME and accent input? | **The press-and-hold accent picker works.** A real CJK IME wasn't tested. |
| Does the PLATFORM_NOTES plan (`show()` + `set_focus()`) work? | **Not needed.** tauri-nspanel's `show_and_make_key` already focuses the webview. The Tauri path wasn't tested. |

## What this means for Kopi

1. **Use tauri-nspanel 2.1 for the macOS panel**, with the setup above. The risk in ADR 0001 is retired. Build the panel once at launch, hidden, and only show and hide it after that. Building it costs 100–800 ms, and showing it costs under 10 ms.
2. **Ship `LSUIElement = true` in Info.plist and keep `set_activation_policy(Accessory)` in `setup`.** You need both: without the first, the Dock icon flashes at launch; without the second, the app runs as Regular.
3. **Show with `show_and_make_key()`, not Tauri's `show()` + `set_focus()`.** In tao 0.37, `set_focus()` calls `makeKeyAndOrderFront` and then `activateIgnoringOtherApps: YES`, which activates Kopi and takes the menu bar from the target app. `show_and_make_key` makes the panel key without activating it, and the webview becomes first responder by itself.
4. **ADR 0004's assumption holds:** the target app never loses activation, so paste can happen right after hiding. Issue #6 checks the ⌘V timing.
5. **Don't use `NSApp.isActive` to ask "is Kopi in front?".** It's true whenever the panel is key. Use `NSWorkspace.frontmostApplication` and compare it to Kopi's own pid instead.
6. **Hide on blur needs a short delay and an exception.** When `Focused(false)` arrives, wait about 100 ms, then skip the hide if the new key window belongs to the Character Palette or to an input method. Otherwise, emoji input from the panel is impossible. While the palette is open, the panel is still visible and Kopi is still not frontmost.
7. **Focus the search input on `panel-shown`, but don't trust `document.hasFocus()` in that handler.** It turns true one frame later. Anything that checks it (for example, "is the panel focused?") should wait for `requestAnimationFrame`, or listen for the window's `focus` event.
8. **Keyboard handling must skip IME keys:** `if (e.isComposing || e.keyCode === 229) return;` before treating Return, ↑/↓ or Esc as Kopi shortcuts. The accent picker sets only `keyCode` 229, so checking `isComposing` alone isn't enough.
9. **Hide on blur should use Tauri's `WindowEvent::Focused`, not a tauri-nspanel event handler.** `panel.set_event_handler()` makes the handler the window's delegate. It keeps tao's delegate only to restore it on `set_event_handler(None)`, and doesn't forward to it. That would stop Tauri's own window events, including `Focused`, for that window.

## Gotchas

- **`set_event_handler` replaces the delegate.** As above: use Tauri's window events, or forward every delegate call yourself.
- **`no_activate(true)`** sets the activation policy to Prohibited while the native window is created, so creating it can't activate the app. It then restores whatever policy was in effect before, so it doesn't change Kopi's own policy. Without `LSUIElement`, Kopi was still active right after the build, because launching the app had already activated it.
- **Rounded corners** come from CSS (`border-radius` on the page's root element) on a transparent window. `transparent(true)` needs `macOSPrivateApi: true` in `tauri.conf.json`, and the `macos-private-api` feature on the `tauri` crate.
- **⇧⌘V is global.** Carbon hotkeys win over every app's own shortcut. So while Kopi runs, ⇧⌘V no longer reaches other apps, such as Chrome's "Paste and Match Style". The shortcut must be configurable (already in MVP_SCOPE), and onboarding should mention the clash.
- **`dist/` is ignored** by the repo's root `.gitignore`. Kopi's scaffold should either build its frontend into a different folder or expect the folder to be ignored.
- **The first show is slower** (about 150 ms to JS) because WebKit wakes up. Later shows reach JS in 6–14 ms. If the first show feels slow, show the panel off-screen once at launch to warm it up.
- **Building without the Tauri CLI** needs `--features tauri/custom-protocol`. Without it, the binary is a dev build, as with `tauri dev`, rather than the release setup `tauri build` produces.

## Not tested

- **More than one display.** Positioning uses the screen with the cursor, but only one display was connected. This matters for issue #7's panel code.
- **A real CJK IME** (Japanese, Chinese, Korean), where Return commits a composition. The `keyCode` 229 guard should cover it, but that hasn't been seen.
- **Tauri's `show()` + `set_focus()`** and the "focus `WKWebView`" mode. The menu has them, but they weren't tried, because the default already worked.
- **Levels 25 and 101.** Level 4 was enough over a full-screen app. Higher levels may be needed over other floating UI.
- **Stage Manager, Mission Control and switching Spaces** while the panel is open.
- **Secure input fields** (for example, a password field focused in the target app). The global hotkey may not fire while secure input is on.
- **VoiceOver.** The panel should announce itself and its focused input.
- **A Developer ID-signed, notarized build.** Only ad-hoc builds were tested.
- **Windows.** Out of scope here. PLATFORM_NOTES already covers it.
