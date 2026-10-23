//! Windows: the Win32 clipboard, the foreground window, `SendInput` and `Windows.Media.Ocr`.
//!
//! The clipboard speaks the macOS format names (UTIs) that the rest of Kopi uses, so the core
//! doesn't know which OS it runs on: `CF_UNICODETEXT` reads as `public.utf8-plain-text`,
//! `HTML Format` as `public.html`, a bitmap as `public.png`, and so on. See
//! [`WinPasteboard`].

mod apps;
mod keys;
mod pasteboard;
mod text;

pub use apps::{
    WinForegroundApp, WinUserIdle, WindowRef, activate, foreground_window, running_apps,
};
pub use keys::{WinKeySender, WinPermissions};
pub use pasteboard::WinPasteboard;
pub(crate) use text::lower_thread_priority;
pub use text::{WinPowerMode, WinRecognizer};

/// `s` as a NUL-terminated UTF-16 string, for the `W` functions.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// UTF-16 up to the first NUL, or all of it.
fn from_wide(s: &[u16]) -> String {
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    String::from_utf16_lossy(&s[..end])
}
