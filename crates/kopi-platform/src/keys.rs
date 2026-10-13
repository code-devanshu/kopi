//! Pasting into the app in front, and the permission that needs.

use thiserror::Error;

/// Sends the paste shortcut to the frontmost app: ⌘V on macOS, Ctrl+V on Windows.
pub trait KeySender: Send + Sync {
    /// Posts the shortcut. On macOS, keys posted without [`Permission::Accessibility`] are
    /// dropped and nothing reports it, so check [`Permissions::is_granted`] first.
    fn send_paste(&self) -> Result<(), KeyError>;
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
}

/// Checks and asks for permissions. Onboarding (#16) adds opening the settings page.
pub trait Permissions: Send + Sync {
    /// Whether the user has granted `permission`. A grant takes effect while Kopi runs.
    fn is_granted(&self, permission: Permission) -> bool;

    /// Shows the system's request for `permission`. Returns at once, before the user answers,
    /// so poll [`is_granted`](Self::is_granted) for the answer.
    fn request(&self, permission: Permission);
}
