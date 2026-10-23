//! Vision reads a fixture image with known text. Runs on macOS only.

#![cfg(target_os = "macos")]

use kopi_platform::macos::VisionRecognizer;
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

#[test]
fn reads_the_lines_of_a_fixture_in_order() {
    let lines = VisionRecognizer.recognize(&fixture("invoice.png")).unwrap();
    assert_eq!(lines, ["Invoice 4821", "Total due: 129 EUR"]);
}

#[test]
fn a_blank_image_has_no_text() {
    let blank = RgbaImage {
        width: 64,
        height: 64,
        pixels: vec![255; 64 * 64 * 4],
    };
    assert_eq!(
        VisionRecognizer.recognize(&blank).unwrap(),
        [] as [String; 0]
    );
}

#[test]
fn pixels_that_dont_match_the_size_are_refused() {
    let wrong = RgbaImage {
        width: 10,
        height: 10,
        pixels: vec![0; 12],
    };
    assert!(matches!(
        VisionRecognizer.recognize(&wrong),
        Err(OcrError::BadImage(10, 10))
    ));
}
