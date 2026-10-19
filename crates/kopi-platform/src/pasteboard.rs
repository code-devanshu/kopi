//! The clipboard, behind a trait that keeps reads in a privacy-safe order: the change count,
//! then the format names, and content only after both have been checked.
//!
//! Neither `change_count` nor `types` reads content, and on macOS neither raises the "Paste from
//! Other Apps" alert (see `docs/spikes/0001-macos-pasteboard-access.md`). `read` is the only
//! call that touches content, and it refuses to run unless the OS allows background reads.

use thiserror::Error;

/// Whether Kopi may read the pasteboard in the background. On macOS 15.4+ this is
/// `NSPasteboard.accessBehavior`, which the user sets in System Settings → Privacy & Security.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteboardAccess {
    /// Never asked. The first read raises the system alert.
    Default,
    /// Every new copy raises the alert again.
    Ask,
    /// Reads work without an alert. Also reported where there's no permission model: macOS
    /// before 15.4, and Windows.
    AlwaysAllow,
    /// Reads return nothing.
    AlwaysDeny,
}

impl PasteboardAccess {
    /// `true` only for [`AlwaysAllow`](Self::AlwaysAllow). Any other state would show an alert
    /// for every copy, and a read waiting on that alert blocks the main thread.
    pub const fn allows_background_reads(self) -> bool {
        matches!(self, Self::AlwaysAllow)
    }
}

/// The format names on the pasteboard, item by item, and the change they belong to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PasteboardTypes {
    pub change_count: u64,
    /// One list per pasteboard item. macOS puts each copied file in its own item.
    pub items: Vec<Vec<String>>,
}

impl PasteboardTypes {
    /// `true` when no item has any format, such as just after another app cleared the
    /// pasteboard and before it wrote anything.
    pub fn is_empty(&self) -> bool {
        self.items.iter().all(Vec::is_empty)
    }

    /// Whether any item has `format`.
    pub fn contains(&self, format: &str) -> bool {
        self.items.iter().flatten().any(|f| f == format)
    }
}

/// One format read from one pasteboard item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFormat {
    pub item_index: u32,
    /// A macOS UTI (`public.rtf`). The Windows clipboard translates its own formats to these.
    pub format: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PasteboardError {
    /// Reading now would raise the system alert or be refused, so nothing was read.
    #[error("background clipboard reads aren't allowed ({0:?})")]
    AccessDenied(PasteboardAccess),
    /// The pasteboard moved on from the change being read, so the data might belong to a
    /// different copy. Nothing is returned.
    #[error("the clipboard changed during the read")]
    Changed,
    /// The formats asked for add up to more than the limit. Nothing is returned.
    #[error("the copy is larger than the limit")]
    TooLarge,
    /// The main thread didn't run the call in time. macOS pasteboard calls must run there.
    #[error("the main thread is busy")]
    MainThreadBusy,
    /// Another app kept the clipboard open. On Windows only one app at a time may open it.
    #[error("another app has the clipboard open")]
    Busy,
    /// The OS turned the write down.
    #[error("couldn't write to the clipboard")]
    WriteFailed,
}

/// The system clipboard. Call `change_count` to spot a change, `types` to see what it holds,
/// and `read` only for the formats the privacy filter allowed. `write` is for pasting.
pub trait Pasteboard: Send {
    /// Goes up on every change of ownership. Reads no content.
    fn change_count(&self) -> Result<u64, PasteboardError>;

    /// The format names of every item, with the change count they were read at. Reads no
    /// content.
    fn types(&self) -> Result<PasteboardTypes, PasteboardError>;

    /// Reads `formats[i]` from item `i`, skipping formats an item doesn't have and items
    /// past the end of `formats`.
    ///
    /// Fails with [`AccessDenied`](PasteboardError::AccessDenied) without reading anything
    /// unless [`access`](Self::access) allows background reads, with
    /// [`Changed`](PasteboardError::Changed) if the pasteboard isn't at `change_count` before
    /// and after the read, and with [`TooLarge`](PasteboardError::TooLarge) once the data
    /// passes `max_bytes`.
    fn read(
        &self,
        change_count: u64,
        formats: &[Vec<String>],
        max_bytes: u64,
    ) -> Result<Vec<RawFormat>, PasteboardError>;

    /// Whether background reads are allowed right now.
    fn access(&self) -> Result<PasteboardAccess, PasteboardError>;

    /// Replaces the clipboard's contents with `formats`, one item per `item_index`, and
    /// returns the new change count. Writing needs no permission.
    ///
    /// `source` names the writer. On macOS it goes in [`uti::SOURCE`] on the first item, which
    /// is how Kopi's watcher knows the write was Kopi's own.
    fn write(&self, formats: &[RawFormat], source: Option<&str>) -> Result<u64, PasteboardError>;
}

/// macOS pasteboard type names (UTIs) that Kopi captures or checks.
pub mod uti {
    pub const PLAIN_TEXT: &str = "public.utf8-plain-text";
    pub const RTF: &str = "public.rtf";
    pub const RTFD: &str = "com.apple.flat-rtfd";
    pub const HTML: &str = "public.html";
    pub const PNG: &str = "public.png";
    pub const TIFF: &str = "public.tiff";
    pub const FILE_URL: &str = "public.file-url";
    pub const URL: &str = "public.url";
    /// An archived `NSColor`.
    pub const COLOR: &str = "com.apple.cocoa.pasteboard.color";

    /// The formats Kopi reads, in the order it prefers them. Anything else is never read.
    pub const CAPTURE: &[&str] = &[PLAIN_TEXT, RTF, RTFD, HTML, PNG, TIFF, FILE_URL, URL, COLOR];

    /// Holds the bundle ID of the app that wrote the pasteboard (nspasteboard.org). Kopi sets
    /// it to its own bundle ID when it pastes, which is how the watcher spots its own writes.
    pub const SOURCE: &str = "org.nspasteboard.source";

    /// Marks a copy that asked not to leave the computer: on Windows,
    /// `CanUploadToCloudClipboard` set to 0. Kopi never syncs it.
    pub const LOCAL_ONLY: &str = "in.devanshuverma.kopi.local-only";
}
