//! Sync between a user's computers, directly over the local network. See `docs/SYNC_DESIGN.md`.
//!
//! - [`discovery`]: finding other computers with DNS-SD over mDNS.
//! - [`pairing`]: adding a computer with a 6-digit code, through SPAKE2.
//! - [`noise`]: encrypted, authenticated connections with pinned keys.
//! - [`session`] and [`protocol`]: exchanging changes, which `kopi-core` merges.
//! - [`SyncService`]: all of it, while sync is on. While it's off, nothing here runs and no
//!   socket is open.
//!
//! Logs have device IDs, counts and errors, never clip content.

pub mod discovery;
pub mod identity;
pub mod noise;
pub mod pairing;
mod probe;
pub mod protocol;
mod service;
pub mod session;

pub use identity::Identity;
pub use service::{
    Config, EventSink, Nearby, PairingStatus, Problem, Status, SyncEvent, SyncService,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Io(#[from] std::io::Error),
    #[error("encryption error: {0}")]
    Noise(String),
    #[error("the other computer sent something unexpected: {0}")]
    Protocol(String),
    #[error("a message of {0} bytes is too large")]
    TooLarge(usize),
    #[error("the other computer isn't paired with this one")]
    Refused,
    #[error("the other computer didn't answer in time")]
    Timeout,
    #[error("the connection closed")]
    Closed,
    #[error("the other computer runs a newer version of Kopi")]
    NeedsUpdate,
    #[error("the code didn't match")]
    WrongCode,
    #[error("a code is 6 digits")]
    BadCode,
    #[error("that computer isn't ready to pair any more")]
    NotPairing,
    #[error("couldn't advertise on the network: {0}")]
    Discovery(String),
    #[error("storage error: {0}")]
    Storage(#[from] kopi_core::storage::StorageError),
}

/// Writes a line to the log. Never pass clip content.
pub(crate) fn log(line: &str) {
    eprintln!("kopi: sync: {line}");
}

/// The name other computers see for this one, until the user changes it: the computer's name.
pub fn default_device_name() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("/usr/sbin/scutil")
            .args(["--get", "ComputerName"])
            .output()
        {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            if out.status.success() && !name.is_empty() {
                return name;
            }
        }
    }
    #[cfg(windows)]
    {
        if let Ok(name) = std::env::var("COMPUTERNAME")
            && !name.trim().is_empty()
        {
            return name.trim().to_owned();
        }
    }
    match kopi_core::current_os() {
        kopi_core::Os::Windows => "Windows PC".to_owned(),
        _ => "Mac".to_owned(),
    }
}
