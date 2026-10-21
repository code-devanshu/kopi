//! Ctrl+V through `SendInput`, and the permissions, which Windows doesn't ask for.

use std::thread;
use std::time::Duration;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_CONTROL, VK_LEFT, VK_SHIFT,
};

use crate::keys::{KeyError, KeySender, Permission, Permissions};

/// V. Virtual keys don't depend on the keyboard layout, so Ctrl+V pastes on every one.
const VK_V: VIRTUAL_KEY = 0x56;

/// How long the window that was just brought to the front gets before the keys arrive.
const FOCUS_SETTLE: Duration = Duration::from_millis(50);

/// Types Ctrl+V into the foreground window.
#[derive(Debug, Clone, Default)]
pub struct WinKeySender;

impl KeySender for WinKeySender {
    fn send_paste(&self) -> Result<(), KeyError> {
        thread::sleep(FOCUS_SETTLE);
        let mut inputs = Vec::new();
        // Shift still held from the shortcut would make it Ctrl+Shift+V, which pastes plain
        // text in browsers. Alt and Win aren't released: on their own, letting go of them
        // opens a menu or Start.
        // SAFETY: takes a key code.
        if unsafe { GetAsyncKeyState(i32::from(VK_SHIFT)) } < 0 {
            inputs.push(key(VK_SHIFT, false));
        }
        inputs.extend([
            key(VK_CONTROL, true),
            key(VK_V, true),
            key(VK_V, false),
            key(VK_CONTROL, false),
        ]);
        send(&inputs)
    }

    fn send_left(&self, count: usize) -> Result<(), KeyError> {
        // Shift or Ctrl still held, such as Ctrl from Ctrl+1, would select or move by words.
        let mut inputs: Vec<INPUT> = [VK_SHIFT, VK_CONTROL]
            .into_iter()
            // SAFETY: takes a key code.
            .filter(|&vk| unsafe { GetAsyncKeyState(i32::from(vk)) } < 0)
            .map(|vk| key(vk, false))
            .collect();
        for _ in 0..count {
            // The arrow keys are extended keys. Without the flag they're the keypad's.
            inputs.push(key_with(VK_LEFT, KEYEVENTF_EXTENDEDKEY));
            inputs.push(key_with(VK_LEFT, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP));
        }
        send(&inputs)
    }
}

fn send(inputs: &[INPUT]) -> Result<(), KeyError> {
    // SAFETY: `inputs` holds `inputs.len()` INPUTs of the size passed.
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent as usize != inputs.len() {
        return Err(KeyError::EventFailed);
    }
    Ok(())
}

fn key(vk: VIRTUAL_KEY, down: bool) -> INPUT {
    key_with(vk, if down { 0 } else { KEYEVENTF_KEYUP })
}

fn key_with(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Windows lets any app read the clipboard and type into other apps, so everything is granted.
/// It won't deliver keys to an app running as administrator, and nothing reports that.
#[derive(Debug, Clone, Default)]
pub struct WinPermissions;

impl Permissions for WinPermissions {
    fn is_granted(&self, _permission: Permission) -> bool {
        true
    }

    fn request(&self, _permission: Permission) {}

    fn open_settings(&self, _permission: Permission) {}
}
