//! Reads the text in image clips, so search finds them. See `docs/OCR_DESIGN.md`.

use std::sync::Arc;

use kopi_core::ocr::{self, Ocr};
use kopi_core::storage::Store;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::events::ImageTextRead;

/// Starts reading new images and the ones already in the history, on macOS. Must run before
/// capture and sync start, so no new image is missed. Elsewhere there's no recognizer yet, and
/// nothing is managed.
pub fn start(app: &AppHandle, store: &Arc<Store>) -> std::io::Result<()> {
    let Some((recognizer, power)) = platform() else {
        return Ok(());
    };
    let handle = app.clone();
    let ocr = Ocr::spawn(
        Arc::clone(store),
        recognizer,
        power,
        ocr::BACKLOG_PAUSE,
        Box::new(move |id| {
            let _ = ImageTextRead { id }.emit(&handle);
        }),
    )?;
    app.manage(ocr);
    Ok(())
}

/// Whether this build can read text in images.
pub fn available(app: &AppHandle) -> bool {
    app.try_state::<Ocr>().is_some()
}

/// The setting was turned on: start on the images in the history.
pub fn resume(app: &AppHandle) {
    if let Some(ocr) = app.try_state::<Ocr>() {
        ocr.resume();
    }
}

type Platform = (
    Box<dyn kopi_platform::TextRecognizer>,
    Box<dyn kopi_platform::PowerMode>,
);

#[cfg(target_os = "macos")]
fn platform() -> Option<Platform> {
    use kopi_platform::macos::{MacPowerMode, VisionRecognizer};

    Some((Box::new(VisionRecognizer), Box::new(MacPowerMode)))
}

#[cfg(not(target_os = "macos"))]
fn platform() -> Option<Platform> {
    None
}
