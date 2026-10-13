//! Kopi's core. It doesn't depend on Tauri, so it can be built and tested on its own. See
//! `docs/ARCHITECTURE.md`.

pub mod classifier;
pub mod paste;
pub mod privacy;
pub mod retention;
pub mod search;
pub mod settings;
pub mod storage;
pub mod watcher;

pub use kopi_platform::{Os, current_os};

/// Kopi's version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
