//! `Windows.Media.Ocr` behind [`TextRecognizer`], Energy saver behind [`PowerMode`], and the
//! thread's background mode.

use std::sync::{Mutex, PoisonError};

use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows_sys::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
};

use crate::text::{OcrError, PowerMode, RgbaImage, TextRecognizer};

/// Larger images are scaled down to this first, as on macOS, even where the engine takes more.
const MAX_DIMENSION: u32 = 4096;

/// The OCR engine for the first language in the user's list that has text recognition
/// installed. There's none until one is, so each check before then tries again, and the engine
/// is kept once made. WinRT starts COM on the thread by itself.
#[derive(Debug, Default)]
pub struct WinRecognizer {
    engine: Mutex<Option<OcrEngine>>,
}

impl WinRecognizer {
    fn engine(&self) -> Option<OcrEngine> {
        let mut engine = self.engine.lock().unwrap_or_else(PoisonError::into_inner);
        if engine.is_none() {
            *engine = OcrEngine::TryCreateFromUserProfileLanguages().ok();
        }
        engine.clone()
    }
}

impl TextRecognizer for WinRecognizer {
    fn available(&self) -> bool {
        self.engine().is_some()
    }

    fn max_dimension(&self) -> u32 {
        OcrEngine::MaxImageDimension().map_or(MAX_DIMENSION, |max| max.min(MAX_DIMENSION))
    }

    fn recognize(&self, image: &RgbaImage) -> Result<Vec<String>, OcrError> {
        let expected = image.width as usize * image.height as usize * 4;
        if image.width == 0 || image.height == 0 || image.pixels.len() != expected {
            return Err(OcrError::BadImage(image.width, image.height));
        }
        let engine = self
            .engine()
            .ok_or_else(|| OcrError::Failed("no language has text recognition".into()))?;
        // SoftwareBitmap takes BGRA.
        let mut bgra = image.pixels.clone();
        for pixel in bgra.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        let failed = |e: windows::core::Error| OcrError::Failed(e.message());
        let writer = DataWriter::new().map_err(failed)?;
        writer.WriteBytes(&bgra).map_err(failed)?;
        let buffer = writer.DetachBuffer().map_err(failed)?;
        // The caller has put the image on white, so it's opaque and premultiplying changes
        // nothing.
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            image.width as i32,
            image.height as i32,
            BitmapAlphaMode::Premultiplied,
        )
        .map_err(failed)?;
        let result = engine
            .RecognizeAsync(&bitmap)
            .and_then(|operation| operation.join())
            .map_err(failed)?;
        result
            .Lines()
            .map_err(failed)?
            .into_iter()
            .map(|line| line.Text().map(|text| text.to_string()).map_err(failed))
            .collect()
    }
}

/// Energy saver, or battery saver before Windows 11 24H2, from `GetSystemPowerStatus`.
#[derive(Debug, Clone, Default)]
pub struct WinPowerMode;

impl PowerMode for WinPowerMode {
    fn saving_power(&self) -> bool {
        // SAFETY: the status is plain data, written by the call.
        unsafe {
            let mut status: SYSTEM_POWER_STATUS = std::mem::zeroed();
            GetSystemPowerStatus(&mut status) != 0 && status.SystemStatusFlag == 1
        }
    }
}

/// Runs the calling thread in background mode: low CPU, I/O and memory priority.
pub(crate) fn lower_thread_priority() {
    // SAFETY: changes only the calling thread, through its pseudo handle.
    unsafe {
        SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
    }
}
