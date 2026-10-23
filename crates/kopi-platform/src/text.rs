//! Reading the text in images with the OS's own recognizer, and what background work needs to
//! stay out of the way: a low thread priority and the power-saving mode.

/// An image as plain pixels: RGBA, 8 bits each, row by row, with no padding. The caller
/// decodes it, so this crate doesn't.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("the image isn't {0} × {1} RGBA pixels")]
    BadImage(u32, u32),
    #[error("text recognition failed: {0}")]
    Failed(String),
}

/// The OS's text recognition. It runs on the computer and needs no network.
pub trait TextRecognizer: Send {
    /// Whether recognition can run here.
    fn available(&self) -> bool;

    /// The longest edge it takes, in pixels. The caller scales larger images down first.
    fn max_dimension(&self) -> u32;

    /// The text in `image`, as lines in reading order. Empty when there's none.
    fn recognize(&self, image: &RgbaImage) -> Result<Vec<String>, OcrError>;
}

/// Whether the user asked the computer to save power: Low Power Mode on macOS.
pub trait PowerMode: Send {
    fn saving_power(&self) -> bool;
}

/// Runs the calling thread at a low priority, for work nobody waits on. It doesn't change
/// threads started from it later.
pub fn lower_thread_priority() {
    #[cfg(target_os = "macos")]
    crate::macos::lower_thread_priority();
}
