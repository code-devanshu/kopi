//! Pasting into the app in front, and the permissions Kopi asks for.

use thiserror::Error;

/// Sends the paste shortcut to the frontmost app: ⌘V on macOS, Ctrl+V on Windows.
pub trait KeySender: Send + Sync {
    /// Posts the shortcut. On macOS, keys posted without [`Permission::Accessibility`] are
    /// dropped and nothing reports it, so check [`Permissions::is_granted`] first.
    fn send_paste(&self) -> Result<(), KeyError>;

    /// Posts the Left arrow `count` times, with no modifiers, to move the cursor back after a
    /// paste. Needs the same permission as [`send_paste`](Self::send_paste).
    fn send_left(&self, count: usize) -> Result<(), KeyError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeyError {
    /// The OS didn't create the key event.
    #[error("couldn't create the key event")]
    EventFailed,
    /// The main thread didn't run the call in time. macOS reads the keyboard layout there.
    #[error("the main thread is busy")]
    MainThreadBusy,
}

/// A permission the user grants in the OS settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// macOS: posting keys into other apps, for pasting.
    Accessibility,
    /// macOS 15.4+: reading the clipboard in the background, "Paste from Other Apps" in
    /// Privacy & Security. Granted where the OS doesn't ask for it.
    PasteFromOtherApps,
}

/// Checks and asks for permissions, and opens the OS settings where they're granted.
pub trait Permissions: Send + Sync {
    /// Whether the user has granted `permission`. A grant takes effect while Kopi runs.
    fn is_granted(&self, permission: Permission) -> bool;

    /// Shows the system's request for `permission`. Returns at once, before the user answers,
    /// so poll [`is_granted`](Self::is_granted) for the answer. The OS has no request for
    /// [`Permission::PasteFromOtherApps`], only its setting, so this does nothing for it.
    fn request(&self, permission: Permission);

    /// Opens the page of the OS settings where the user grants `permission`.
    fn open_settings(&self, permission: Permission);
}
