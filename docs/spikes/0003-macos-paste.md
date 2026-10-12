# Spike 0003: paste into the active app on macOS

- **Issue:** [#6](https://github.com/code-devanshu/kopi/issues/6)
- **Date:** 2026-10-13
- **Code:** [spikes/paste](../../spikes/paste/README.md)
- **Feeds into:** [ADR 0004](../decisions/0004-paste-simulation-approach.md), [PLATFORM_NOTES.md](../PLATFORM_NOTES.md#pasting-into-the-active-app)

## Question

Can Kopi paste a clip into the app the user was in, by writing the clip to the pasteboard, hiding the panel and posting ⌘V with `CGEvent`? In particular:

- What happens without Accessibility, and how do we ask for it?
- Which key code is V on the current layout, including Dvorak and "Dvorak – QWERTY ⌘"?
- Do modifiers still held from the hotkey (⇧ from ⇧⌘V) get in the way?
- How long do we have to wait between hiding the panel and posting?
- Which apps does it work in, and where does it fail?

## Setup

| | |
| --- | --- |
| macOS | 27.0.1 (26A434), one display, ABC – India keyboard layout |
| Stack | Tauri 2.12.3, tauri-nspanel 2.1.1, tauri-plugin-global-shortcut 2.4.1, objc2 0.6, Rust 1.99.0 |
| App | `in.devanshuverma.kopi.pastespike` 0.1.0 (1), release build, ad-hoc signed, not sandboxed, `LSUIElement` |
| Panel | The non-activating panel from [spike 0002](0002-macos-nspanel.md), shown with `show_and_make_key()`. The target app stays frontmost while the panel is open. |
| Triggers | ⇧⌘V opens the panel, and Return pastes the selected clip. ⇧⌘1 pastes the first clip at once, while ⇧⌘ are still held. |

A paste runs on the main thread, in this order:

1. Look up the key code for V in the current layout (see [Key codes](#key-codes)).
2. Write the clip to the general pasteboard: `clearContents`, then the plain text, `public.html` for rich clips, and `org.nspasteboard.source` set to the app's bundle ID.
3. Hide the panel.
4. Post V down and V up:

```rust
let source = CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
// Copied from Maccy: hold back local keyboard events while ours are posted.
CGEventSourceSetLocalEventsFilterDuringSuppressionState(
    source,
    kCGEventFilterMaskPermitLocalMouseEvents | kCGEventFilterMaskPermitSystemDefinedEvents,
    kCGEventSuppressionStateSuppressionInterval,
);
for down in [true, false] {
    let event = CGEventCreateKeyboardEvent(source, key, down);
    CGEventSetFlags(event, kCGEventFlagMaskCommand | NX_DEVICELCMDKEYMASK); // 0x100008
    CGEventPost(kCGSessionEventTap, event);
}
```

The menu bar icon can switch the key code, a delay after hiding (0–250 ms), the keystroke sequence, the flags, the event source, the tap, and waiting for modifiers to be released. The defaults worked in every app tested, so none of the alternatives were needed.

Fifty milliseconds after each paste, the spike asks the Accessibility API for the focused app and element, as a rough check of where the keys went.

## Results

### Permission

| Check | Result |
| --- | --- |
| Paste without Accessibility (3 tries in Notes) | **Nothing is pasted, and nothing reports an error.** `CGEventPost` returns nothing, the events are created normally, and the target is frontmost. The events are silently dropped. |
| `AXIsProcessTrusted()` without the grant | false |
| `AXIsProcessTrustedWithOptions` with `kAXTrustedCheckOptionPrompt` | Returns **false at once** and shows the system alert, which sends the user to System Settings. |
| Grant in System Settings while the app runs | `AXIsProcessTrusted()` turned true within one second (it's polled every second). **No relaunch needed.** The next paste worked. |
| `CGPreflightPostEventAccess()` | **false the whole time**, including while pastes worked. |
| `IsSecureEventInputEnabled()` | false in every test |

### Apps

All with the default settings: key code from the layout with ⌘ held (9), no delay, V down/up with flags `0x100008`, session tap.

| # | App, field | Clip | Posted after hiding | Trigger to posted | Focused element at +50 ms | Result |
| --- | --- | --- | --- | --- | --- | --- |
| 4 | Notes, note body | Plain text | 0.5 ms | 11.7 ms | AXTextArea | **Pasted** |
| 5 | TextEdit, document | Plain text | 0.6 ms | 8.8 ms | AXTextArea | **Pasted** |
| 6 | Safari, address bar | Plain text | 1.2 ms | 8.9 ms | AXTextField | **Pasted** |
| 7 | Chrome, address bar | Plain text | 0.9 ms | 7.3 ms | AXTextField | **Pasted** |
| 8 | VS Code, untitled file | Plain text | 1.1 ms | 8.4 ms | AXTextArea | **Pasted** |
| 9 | Terminal, prompt | `echo kopi-paste-ok-9` | 0.8 ms | 11.4 ms | AXTextArea | **Pasted, not run.** No newline was added, so the user still has to press Return. |
| 10 | TextEdit, via ⇧⌘1 | Plain text | 1.4 ms | 11.4 ms | AXTextArea | **Pasted**, with ⇧⌘ still held (see below) |
| 11 | TextEdit, rich document | Rich text (HTML) | 0.6 ms | 7.4 ms | error −25204 | **Pasted, formatting kept**: bold, italics and the link |
| 12 | Notes, note body | Rich text (HTML) | 0.7 ms | 9.4 ms | error −25204 | **Pasted, formatting kept** |

For a Return paste, the trigger is the moment the page's `pick` command arrives. For ⇧⌘1, it's the hotkey.

In every paste, the frontmost app at the moment of posting was the target.

Slack wasn't tested, because it isn't installed. VS Code is the only Electron app in the table.

### Key codes

The spike scanned all 264 keyboard layouts installed with macOS 27.0.1, using `UCKeyTranslate` with no modifiers, and again with ⌘ held. In each pass, it looked for the key that types "v".

| Layouts | V with no modifiers | V with ⌘ held |
| --- | --- | --- |
| 136 Latin layouts (ABC, US, British, French AZERTY, German, Czech, Turkish Q, Dvorak – Left-Handed, …) | 9 | 9 |
| 122 non-Latin layouts (Arabic, Cyrillic, Greek, Hebrew, Indic, Thai, …) | **none** | 9 |
| Dvorak | 47 | 47 |
| **Dvorak – QWERTY ⌘** | **47** | **9** |
| Dvorak – Right-Handed | 43 | 43 |
| Turkish F, Turkish F – Legacy | 8 | 8 |
| Turkmen | none | none |

Only the current layout was used for live pastes. That was ABC – India, where V is 9 either way. The other layouts were scanned, not typed on.

### Held modifiers

| Check | Result |
| --- | --- |
| Return paste (#4–#9, #11, #12) | Modifiers held just before posting: **none**. **Return was still down** every time. |
| ⇧⌘1 paste (#10) | Modifiers held just before posting: **⇧⌘**. The V key-down still carried only `0x100008` (⌘ plus the left-⌘ device bit), because `CGEventSetFlags` replaces the flags. TextEdit pasted normally. |

### Timing

- With no delay, ⌘V was posted **0.5–1.4 ms** after the panel was hidden, and **7–12 ms** after the trigger.
- No app missed the paste, so no delay setting was needed. The panel never activates Kopi, so the target app is already frontmost and key when the panel hides.

### Checking where the keys went

The 50 ms Accessibility check named the right app and a text element after every plain-text paste. After both rich-text pastes, it failed with `kAXErrorCannotComplete` (−25204) after about 300 ms. That's the 250 ms messaging timeout, because the app was still busy converting the HTML. Both pastes worked. Without the Accessibility grant, the check always fails with the same error.

## Answers

| Question | Answer |
| --- | --- |
| Does write + hide + ⌘V work? | **Yes**, in Notes, TextEdit, Safari, Chrome, VS Code and Terminal, with no delay. Rich text keeps its formatting in TextEdit and Notes. |
| What happens without Accessibility? | **The ⌘V is silently dropped.** No API call fails, so Kopi has to check `AXIsProcessTrusted()` itself before posting. |
| How do we ask? | `AXIsProcessTrustedWithOptions` with the prompt option, which shows the system alert. The grant takes effect while Kopi runs, with no relaunch. |
| Can `CGPreflightPostEventAccess` be the check? | **No.** It stayed false while Accessibility was granted and pastes worked. |
| Which key code is V? | **Look V up in the current layout with ⌘ held, and fall back to 9.** That gives 9 on Latin and non-Latin layouts and on "Dvorak – QWERTY ⌘", 47 on Dvorak, 43 on Dvorak – Right-Handed, and 8 on Turkish F. A lookup with no modifiers gives the wrong key on "Dvorak – QWERTY ⌘", and no key on 122 layouts. |
| Do held modifiers get in the way? | **No**, as long as the flags are set explicitly. Waiting for the user to release ⇧⌘ isn't needed. |
| Delay between hiding and pasting? | **None needed** with the non-activating panel. Post right after hiding. |
| Where does it fail? | **Nowhere in the apps tested.** Slack, other Electron apps, and secure input fields weren't tested. |

## What this means for Kopi

1. **ADR 0004 holds as written.** Use the recipe above: a combined-session event source, V down and up with the flags set to `⌘ | 0x8`, posted to the session event tap. Post right after hiding the panel, with no delay and no wait for modifiers.
2. **Check `AXIsProcessTrusted()` before every paste.** If it's false, leave the clip on the pasteboard and show "Copied. Press ⌘V". Posting without the grant fails silently, so the check is the only way to know. Don't use `CGPreflightPostEventAccess()` for this.
3. **Onboarding asks with `AXIsProcessTrustedWithOptions` (prompt), then polls `AXIsProcessTrusted()` about once a second** until it turns true. The grant works at once, so onboarding can move on without asking the user to relaunch Kopi.
4. **Find V with `UCKeyTranslate` on `TISCopyCurrentKeyboardLayoutInputSource()`, with ⌘ held** (modifier state `cmdKey >> 8`, which is 1). Scan key codes 0–127 for "v", and fall back to 9 if nothing matches. Do this on the main thread, at paste time, because the user can switch layouts at any moment. The scan takes well under a millisecond.
5. **Write rich formats alongside plain text.** Plain text plus `public.html` kept the formatting in TextEdit and Notes. Kopi should write every format it stored for the clip, not only the plain text.
6. **Kopi can't tell whether a paste landed.** No API reports it, and the Accessibility check afterwards times out while apps process rich text. Don't try to verify pastes. If Kopi ever needs to, check that the frontmost app is still the target just before posting, and log a mismatch.
7. **Terminal doesn't run pasted commands**, because the text has no trailing newline. Kopi should keep clips exactly as copied, and never add a newline.

## Gotchas

- **Return is still down when ⌘V is posted.** The key-down went to the panel, so the target app gets only the Return key-up. No app reacted to it, and Terminal didn't run the command. Keep an eye on this in apps that act on key-up.
- **`AXIsProcessTrustedWithOptions(prompt)` returns before the user answers.** It returns false and shows the alert, so its result is never the answer. Poll for the change instead.
- **Every ad-hoc build loses the Accessibility grant**, because TCC ties it to the code signature. The spike puts every option in the menu bar so that testing doesn't need a rebuild. Kopi's dev builds need a stable signing identity (already in PLATFORM_NOTES).
- **Text Input Sources must be read on the main thread.** The spike reads the layout before the paste leaves the main thread.
- **The ⇧⌘1 test can't fully tell ⌘V from ⇧⌘V in TextEdit.** With a plain-text clip, "Paste and Match Style" looks the same as Paste. The evidence that the app got a plain ⌘V is the flags on the event (`0x100008`, no ⇧).
- **The local-events filter is copied from Maccy and wasn't compared** against posting without it.

## Not tested

- **Slack**, and Electron apps other than VS Code.
- **A live paste with Dvorak or "Dvorak – QWERTY ⌘" selected.** Those layouts were scanned, not used.
- **An input method as the current input source** (Japanese, Chinese, Korean). `TISCopyCurrentKeyboardLayoutInputSource` should return the layout under the input method, but that wasn't checked.
- **Secure input**, for example a password field focused in the target app. ⌘V into a password field should still work, but the global hotkey may not fire.
- **The plain multi-line, Unicode and 5,000-character clips.** They're in the spike, but the core tests used the short plain clip, the shell command and the rich clip.
- **Images and files on the pasteboard.** The spike only writes text and HTML.
- **The alternative settings**: delays, ⌘ down/up around V, other flags, event sources and taps, waiting for modifiers. The defaults worked everywhere, so they weren't needed.
- **Restoring the user's previous clipboard after pasting.** That's a possible later setting in ADR 0004.
- **Full-screen apps and more than one display.** Spike 0002 showed the panel over a full-screen app, but nothing was pasted there.
- **Whether the Accessibility grant survives an app update** of a Developer ID-signed build.
