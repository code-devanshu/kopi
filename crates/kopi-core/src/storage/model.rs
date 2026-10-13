//! The plain types that go in and out of the store.

use std::fmt;
use std::str::FromStr;

use super::StorageError;

/// What a clip is. Stored in `clips.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipKind {
    Text,
    RichText,
    Link,
    Image,
    File,
    Color,
    Code,
}

impl ClipKind {
    /// Every kind, in the order of the enum.
    pub const ALL: [Self; 7] = [
        Self::Text,
        Self::RichText,
        Self::Link,
        Self::Image,
        Self::File,
        Self::Color,
        Self::Code,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::RichText => "rich_text",
            Self::Link => "link",
            Self::Image => "image",
            Self::File => "file",
            Self::Color => "color",
            Self::Code => "code",
        }
    }
}

impl fmt::Display for ClipKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ClipKind {
    type Err = StorageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "text" => Self::Text,
            "rich_text" => Self::RichText,
            "link" => Self::Link,
            "image" => Self::Image,
            "file" => Self::File,
            "color" => Self::Color,
            "code" => Self::Code,
            other => {
                return Err(StorageError::Corrupt(format!(
                    "unknown clip kind {other:?}"
                )));
            }
        })
    }
}

/// The clipboard a format came from. Stored in `clip_formats.platform`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    MacOs,
    Windows,
}

impl Platform {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Windows => "windows",
        }
    }

    /// The format name that holds plain text on this platform.
    pub const fn plain_text_format(self) -> &'static str {
        match self {
            Self::MacOs => "public.utf8-plain-text",
            Self::Windows => "CF_UNICODETEXT",
        }
    }
}

impl FromStr for Platform {
    type Err = StorageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "macos" => Ok(Self::MacOs),
            "windows" => Ok(Self::Windows),
            other => Err(StorageError::Corrupt(format!("unknown platform {other:?}"))),
        }
    }
}

/// One raw representation that was on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFormat {
    /// The pasteboard item it came from. macOS can hold several, such as one per file.
    pub item_index: u32,
    /// A macOS UTI (`public.rtf`) or a Windows format name (`HTML Format`).
    pub format: String,
    pub data: Vec<u8>,
}

/// A clip as the classifier hands it to the store.
#[derive(Debug, Clone, PartialEq)]
pub struct NewClip {
    pub kind: ClipKind,
    /// Display and search text. Anything over [`MAX_TEXT_INDEX_BYTES`](super::MAX_TEXT_INDEX_BYTES)
    /// is cut at a character boundary.
    pub plain_text: Option<String>,
    /// BLAKE3 hex of the kind and the canonical payload. Copies with the same hash are one clip.
    pub content_hash: String,
    pub source_app_name: Option<String>,
    pub source_app_id: Option<String>,
    /// `{w,h}` for images, `{rgba}` for colors, `{host}` for links and so on.
    pub metadata: Option<serde_json::Value>,
    pub platform: Platform,
    pub formats: Vec<NewFormat>,
}

/// What [`Store::upsert_clip`](super::Store::upsert_clip) did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upserted {
    pub id: i64,
    pub uuid: String,
    /// `true` for a new clip, `false` when an existing one moved to the top.
    pub inserted: bool,
}

/// A row of `clips`.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub id: i64,
    pub uuid: String,
    pub kind: ClipKind,
    pub plain_text: Option<String>,
    pub content_hash: String,
    pub size_bytes: i64,
    pub source_app_name: Option<String>,
    pub source_app_id: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
    pub use_count: i64,
    pub pinned: bool,
    pub metadata: Option<serde_json::Value>,
}

/// A stored format, with its data loaded from the row or from the blob file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipFormat {
    pub item_index: u32,
    pub platform: Platform,
    pub format: String,
    pub data: Vec<u8>,
}

/// A row of `excluded_apps`: an app Kopi never captures from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludedApp {
    pub platform: Platform,
    /// A macOS bundle ID, or a Windows exe path.
    pub app_id: String,
    pub app_name: String,
    /// `true` for the defaults Kopi seeded, `false` for apps the user added.
    pub is_default: bool,
    pub created_at: i64,
}
