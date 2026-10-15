//! Ctrl+V through `SendInput`, and the permissions, which Windows doesn't ask for.

use std::thread;
use std::time::Duration;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_CONTROL, VK_SHIFT,
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
}

fn key(vk: VIRTUAL_KEY, down: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if down { 0 } else { KEYEVENTF_KEYUP },
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
