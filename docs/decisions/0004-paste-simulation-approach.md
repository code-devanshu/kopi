# 0004. Paste by writing the clipboard and simulating ⌘V / Ctrl+V

- **Status:** Accepted. Verified on macOS in [spike 0003](../spikes/0003-macos-paste.md).
- **Date:** 2026-10-11

## Context

The core action is: press the hotkey, pick a clip, and have it appear in the app you were using. Operating systems have no "insert this into the focused field of another app" API. Every popular clipboard manager (Maccy, Paste, Raycast, Ditto) writes the clipboard and then sends the paste shortcut.

## Decision

1. Remember the target app when the hotkey fires.
2. Write the clip's stored formats to the clipboard (or plain text only for "paste as plain text"), marked as Kopi's own write.
3. Hide the panel and make sure the target is frontmost.
   - On macOS, the panel is a non-activating `NSPanel`, so the target never lost activation.
   - On Windows, call `SetForegroundWindow` on the remembered `HWND`.
4. Send the paste keystroke:
   - **macOS:** `CGEvent` key down/up for V with the flags set explicitly to ⌘, posted to the session event tap right after hiding, with no delay. The key code comes from the current keyboard layout with ⌘ held, falling back to 9. Needs **Accessibility** permission.
   - **Windows:** `SendInput` with Ctrl+V, releasing any modifiers that are still held. No permission is needed, but it can't reach elevated apps (UIPI).
5. If the permission is missing, or the target can't receive input, leave the clip on the clipboard and show "Copied. Press ⌘V / Ctrl+V". On macOS, check `AXIsProcessTrusted()` before posting, because events posted without the permission are dropped silently.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Accessibility API to set the focused element's value (`AXUIElementSetAttributeValue`) | Plain text only, breaks undo, doesn't work in many apps (web views, Electron, terminals), and still needs Accessibility. |
| Typing the text with synthetic key events | Slow for long text, breaks with input methods and autocorrect, and can't paste images or files. |
| Copy only, the user presses ⌘V | Always works, but it's a worse experience. Kept as the fallback, not the default. |
| AppleScript `System Events` keystroke | Needs the Automation permission as well, and it's slower. No gain over CGEvent. |

## Consequences

- **Good:** works in nearly every app, pastes rich text, images and files, and behaves the way users expect from other clipboard managers.
- **Bad:** macOS needs the Accessibility permission, which is a big ask, so onboarding explains it and the app works without it. Pasting overwrites the user's current clipboard. Restoring it afterwards is a possible later setting. On macOS, spike 0003 found that no delay is needed between hiding the panel and the keystroke. Windows timing is still to be checked.
- **Constraint:** Kopi can't ship in the Mac App Store (sandbox plus Accessibility). It's distributed with Developer ID and notarization.
