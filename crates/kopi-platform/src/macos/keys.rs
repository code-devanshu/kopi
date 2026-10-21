//! ⌘V through `CGEvent`, and the permission checks. The paste recipe is the one spike 0003
//! tested (`docs/spikes/0003-macos-paste.md`).

use std::ffi::c_void;

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use super::{MacPasteboard, on_main};
use crate::keys::{KeyError, KeySender, Permission, Permissions};
use crate::pasteboard::Pasteboard;

/// Posts ⌘V to the session event tap, which delivers it to the frontmost app.
#[derive(Debug, Clone, Default)]
pub struct MacKeySender;

impl KeySender for MacKeySender {
    fn send_paste(&self) -> Result<(), KeyError> {
        // Text Input Sources must be read on the main thread, and the user can switch layouts
        // at any moment, so V is looked up for each paste.
        on_main(|| post_command(v_key_code())).ok_or(KeyError::MainThreadBusy)?
    }

    fn send_left(&self, count: usize) -> Result<(), KeyError> {
        // Arrow keys are the same on every layout, so no main-thread lookup is needed.
        post_keys(LEFT_ARROW, FLAG_ARROW, count)
    }
}

/// `AXIsProcessTrusted` and its prompt, and the pasteboard's `accessBehavior`.
#[derive(Debug, Clone, Default)]
pub struct MacPermissions;

impl Permissions for MacPermissions {
    fn is_granted(&self, permission: Permission) -> bool {
        match permission {
            // SAFETY: takes nothing and returns a Boolean. Safe from any thread.
            Permission::Accessibility => unsafe { AXIsProcessTrusted() != 0 },
            Permission::PasteFromOtherApps => MacPasteboard::general()
                .access()
                .is_ok_and(|access| access.allows_background_reads()),
        }
    }

    fn request(&self, permission: Permission) {
        match permission {
            Permission::Accessibility => {
                on_main(|| {
                    // SAFETY: a CFString constant, toll-free bridged to NSString.
                    let key = unsafe { &*kAXTrustedCheckOptionPrompt };
                    let options = NSDictionary::from_slices(&[key], &[&*NSNumber::new_bool(true)]);
                    // Shows the system alert and returns false straight away. The answer
                    // comes later, through `is_granted`.
                    // SAFETY: `options` is a valid dictionary for the duration of the call.
                    unsafe { AXIsProcessTrustedWithOptions(&*options as *const _ as Ref) };
                });
            }
            // Apple asks apps to send the user to System Settings instead.
            Permission::PasteFromOtherApps => {}
        }
    }

    fn open_settings(&self, permission: Permission) {
        let url = match permission {
            Permission::Accessibility => ACCESSIBILITY_SETTINGS,
            Permission::PasteFromOtherApps => PASTEBOARD_SETTINGS,
        };
        on_main(move || {
            if let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) {
                NSWorkspace::sharedWorkspace().openURL(&url);
            }
        });
    }
}

/// System Settings → Privacy & Security → Accessibility.
const ACCESSIBILITY_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
/// System Settings → Privacy & Security → Paste from Other Apps. An anchor System Settings
/// doesn't know opens Privacy & Security, which lists it.
const PASTEBOARD_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Pasteboard";

/// ⌘ in `CGEventFlags`.
const FLAG_COMMAND: u64 = 0x0010_0000;
/// `NX_DEVICELCMDKEYMASK`: the left ⌘ key. Set as well, as Maccy does.
const FLAG_LEFT_COMMAND: u64 = 0x0000_0008;
/// `kCGEventSourceStateCombinedSessionState`
const COMBINED_SESSION_STATE: i32 = 0;
/// `kCGSessionEventTap`
const SESSION_EVENT_TAP: u32 = 1;
/// `kCGEventFilterMaskPermitLocalMouseEvents | kCGEventFilterMaskPermitSystemDefinedEvents`
const PERMIT_MOUSE_AND_SYSTEM_EVENTS: u32 = 0x1 | 0x4;
/// `kCGEventSuppressionStateSuppressionInterval`
const SUPPRESSION_INTERVAL: u32 = 0;
/// `NX_NUMERICPADMASK | NX_SECONDARYFNMASK`: the flags macOS sets on a real arrow key. Setting
/// only these also clears whatever the user is holding, so ⇧ doesn't turn the moves into a
/// selection.
const FLAG_ARROW: u64 = 0x0020_0000 | 0x0080_0000;
/// V on an ANSI keyboard, and on every Latin layout with ⌘ held.
const ANSI_V: u16 = 9;
/// `kVK_LeftArrow`
const LEFT_ARROW: u16 = 0x7B;
/// `cmdKey >> 8`, the modifier state `UCKeyTranslate` takes.
const UC_COMMAND: u32 = 1;
/// `kUCKeyActionDown`
const UC_KEY_DOWN: u16 = 0;
/// `kUCKeyTranslateNoDeadKeysMask`
const UC_NO_DEAD_KEYS: u32 = 1;

/// Posts V down and up with only ⌘ in the flags. Setting the flags replaces whatever the user
/// is still holding, such as ⇧⌘ from a hotkey.
fn post_command(key: u16) -> Result<(), KeyError> {
    post_keys(key, FLAG_COMMAND | FLAG_LEFT_COMMAND, 1)
}

/// Posts `key` down and up `count` times with exactly `flags`.
fn post_keys(key: u16, flags: u64, count: usize) -> Result<(), KeyError> {
    // SAFETY: CoreGraphics calls on objects created here and released before returning.
    unsafe {
        let source = CGEventSourceCreate(COMBINED_SESSION_STATE);
        if !source.is_null() {
            // Holds back the user's own key presses while ours go through.
            CGEventSourceSetLocalEventsFilterDuringSuppressionState(
                source,
                PERMIT_MOUSE_AND_SYSTEM_EVENTS,
                SUPPRESSION_INTERVAL,
            );
        }
        let mut result = Ok(());
        'presses: for _ in 0..count {
            for down in [true, false] {
                let event = CGEventCreateKeyboardEvent(source, key, down);
                if event.is_null() {
                    result = Err(KeyError::EventFailed);
                    break 'presses;
                }
                CGEventSetFlags(event, flags);
                CGEventPost(SESSION_EVENT_TAP, event);
                CFRelease(event);
            }
        }
        if !source.is_null() {
            CFRelease(source);
        }
        result
    }
}

/// The key that types "v" with ⌘ held in the current layout, or [`ANSI_V`]. With ⌘ held,
/// "Dvorak – QWERTY ⌘" gives 9 and non-Latin layouts find V at all. Main thread only.
fn v_key_code() -> u16 {
    // SAFETY: Text Input Source calls on the main thread. The layout data belongs to `source`,
    // which is released only after the last use of it.
    unsafe {
        let source = TISCopyCurrentKeyboardLayoutInputSource();
        if source.is_null() {
            return ANSI_V;
        }
        let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        let key = if data.is_null() {
            None
        } else {
            find_key(CFDataGetBytePtr(data), 'v')
        };
        CFRelease(source);
        key.unwrap_or(ANSI_V)
    }
}

/// Scans key codes 0–127 for the one that types `ch` with ⌘ held.
///
/// # Safety
/// `layout` must point to a `'uchr'` keyboard layout that stays valid for the call.
unsafe fn find_key(layout: *const u8, ch: char) -> Option<u16> {
    // SAFETY: no arguments.
    let keyboard_type = u32::from(unsafe { LMGetKbdType() });
    (0..128u16).find(|&key| {
        let mut dead_keys = 0u32;
        let mut len = 0usize;
        let mut chars = [0u16; 4];
        // SAFETY: `layout` is valid (the caller's promise) and the out-pointers are locals.
        let status = unsafe {
            UCKeyTranslate(
                layout,
                key,
                UC_KEY_DOWN,
                UC_COMMAND,
                keyboard_type,
                UC_NO_DEAD_KEYS,
                &mut dead_keys,
                chars.len(),
                &mut len,
                chars.as_mut_ptr(),
            )
        };
        status == 0 && len == 1 && u32::from(chars[0]) == ch as u32
    })
}

type Ref = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> u8;
    static kAXTrustedCheckOptionPrompt: *const NSString;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceCreate(state_id: i32) -> Ref;
    fn CGEventSourceSetLocalEventsFilterDuringSuppressionState(
        source: Ref,
        filter: u32,
        state: u32,
    );
    fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventPost(tap: u32, event: Ref);
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: Ref;
    fn TISCopyCurrentKeyboardLayoutInputSource() -> Ref;
    fn TISGetInputSourceProperty(source: Ref, key: Ref) -> Ref;
    fn LMGetKbdType() -> u8;
    #[allow(clippy::too_many_arguments)]
    fn UCKeyTranslate(
        layout: *const u8,
        key: u16,
        action: u16,
        modifiers: u32,
        keyboard_type: u32,
        options: u32,
        dead_key_state: *mut u32,
        max: usize,
        actual: *mut usize,
        chars: *mut u16,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: Ref);
    fn CFDataGetBytePtr(data: Ref) -> *const u8;
}
