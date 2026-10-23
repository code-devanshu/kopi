//! `Windows.Media.Ocr` reads a fixture image with known text. Runs on Windows only, and skips
//! itself where no language has text recognition installed.

#![cfg(target_os = "windows")]

use kopi_platform::windows::WinRecognizer;
use kopi_platform::{OcrError, RgbaImage, TextRecognizer};

fn fixture(name: &str) -> RgbaImage {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let image = image::open(path).unwrap().into_rgba8();
    RgbaImage {
        width: image.width(),
        height: image.height(),
        pixels: image.into_raw(),
    }
}

/// The recognizer, or None with a note when this computer has no OCR language.
fn recognizer() -> Option<WinRecognizer> {
    let recognizer = WinRecognizer::default();
    if recognizer.available() {
        Some(recognizer)
    } else {
        eprintln!("skipped: no language here has text recognition installed");
        None
    }
}

#[test]
fn reads_the_lines_of_a_fixture_in_order() {
    let Some(recognizer) = recognizer() else {
        return;
    };
    let lines = recognizer.recognize(&fixture("invoice.png")).unwrap();
    // The engine's spacing around punctuation isn't promised, so the words are checked.
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines[0].starts_with("Invoice") && lines[0].contains("4821"),
        "{lines:?}"
    );
    assert!(
        lines[1].contains("Total") && lines[1].contains("129 EUR"),
        "{lines:?}"
    );
}

#[test]
fn a_blank_image_has_no_text() {
    let Some(recognizer) = recognizer() else {
        return;
    };
    let blank = RgbaImage {
        width: 64,
        height: 64,
        pixels: vec![255; 64 * 64 * 4],
    };
    assert_eq!(recognizer.recognize(&blank).unwrap(), [] as [String; 0]);
}

#[test]
fn pixels_that_dont_match_the_size_are_refused() {
    let wrong = RgbaImage {
        width: 10,
        height: 10,
        pixels: vec![0; 12],
    };
    assert!(matches!(
        WinRecognizer::default().recognize(&wrong),
        Err(OcrError::BadImage(10, 10))
    ));
}
