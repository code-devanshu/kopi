//! The only crate that calls OS APIs. Everything above it works with plain Rust types.
//!
//! `Pasteboard`, `ForegroundApp` and `UserIdle` are here. `KeySender` and `Permissions` arrive
//! with the features that need them. See `docs/ARCHITECTURE.md`.

mod apps;
mod pasteboard;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub use apps::{AppRef, ForegroundApp, UserIdle};
pub use pasteboard::{
    Pasteboard, PasteboardAccess, PasteboardError, PasteboardTypes, RawFormat, uti,
};

/// The operating system this build targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Windows,
    Other,
}

/// The operating system this build targets.
pub const fn current_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::MacOs
    } else if cfg!(target_os = "windows") {
        Os::Windows
    } else {
        Os::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_os_matches_target() {
        let expected = if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Other
        };
        assert_eq!(current_os(), expected);
    }
}
