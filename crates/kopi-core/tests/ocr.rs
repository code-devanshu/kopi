//! Text in images: the store's side and the reading thread, with a fake recognizer.

use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kopi_core::classifier::content_hash;
use kopi_core::ocr::Ocr;
use kopi_core::storage::{
    ClipKind, IMAGE_TEXT_READER, NewClip, NewFormat, Platform, Store, now_ms,
};
use kopi_platform::{OcrError, PowerMode, RgbaImage, TextRecognizer};

/// How long a test waits for the thread before failing.
const WAIT: Duration = Duration::from_secs(10);

/// Reads every image as "image <width>", and fails for the widths in `fail`. With `missing`
/// set, it can't run, as on Windows without an OCR language.
#[derive(Clone, Default)]
struct FakeRecognizer {
    read: Arc<Mutex<Vec<u32>>>,
    fail: Vec<u32>,
    missing: Arc<AtomicBool>,
}

impl TextRecognizer for FakeRecognizer {
    fn available(&self) -> bool {
        !self.missing.load(Ordering::SeqCst)
    }
    fn max_dimension(&self) -> u32 {
        4096
    }
    fn recognize(&self, image: &RgbaImage) -> Result<Vec<String>, OcrError> {
        self.read.lock().unwrap().push(image.width);
        if self.fail.contains(&image.width) {
            return Err(OcrError::Failed("fake".into()));
        }
        Ok(vec![format!("  image {}  ", image.width), String::new()])
    }
}

#[derive(Clone, Default)]
struct FakePower(Arc<AtomicBool>);

impl PowerMode for FakePower {
    fn saving_power(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    store: Arc<Store>,
    recognizer: FakeRecognizer,
    power: FakePower,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path()).unwrap());
        Self {
            _dir: dir,
            store,
            recognizer: FakeRecognizer::default(),
            power: FakePower::default(),
        }
    }

    /// Starts reading, with `pause` between backlog images. The receiver gets each id written.
    fn start(&self, pause: Duration) -> (Ocr, Receiver<i64>) {
        let (tx, rx) = mpsc::channel();
        let ocr = Ocr::spawn(
            Arc::clone(&self.store),
            Box::new(self.recognizer.clone()),
            Box::new(self.power.clone()),
            pause,
            Box::new(move |id| {
                let _ = tx.send(id);
            }),
        )
        .unwrap();
        (ocr, rx)
    }

    /// Stores a PNG `width` pixels wide, used at `now`.
    fn image(&self, width: u32, now: i64) -> i64 {
        let png = png(width, 20);
        let clip = NewClip {
            kind: ClipKind::Image,
            plain_text: None,
            content_hash: content_hash(ClipKind::Image, &png),
            source_app_name: Some("Preview".into()),
            source_app_id: Some("com.apple.Preview".into()),
            metadata: None,
            platform: Platform::MacOs,
            formats: vec![NewFormat {
                item_index: 0,
                format: "public.png".into(),
                data: png,
            }],
            local_only: false,
        };
        self.store.upsert_clip_at(clip, now).unwrap().id
    }

    fn text_of(&self, id: i64) -> (Option<String>, Option<u32>) {
        let clip = self.store.get_clip(id).unwrap().unwrap();
        (clip.plain_text, clip.ocr_version)
    }

    fn widths_read(&self) -> Vec<u32> {
        self.recognizer.read.lock().unwrap().clone()
    }
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut png = Vec::new();
    image::RgbaImage::new(width, height)
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    png
}

fn next(rx: &Receiver<i64>) -> i64 {
    rx.recv_timeout(WAIT).expect("no image was read in time")
}

#[test]
fn a_new_image_is_read_and_its_text_written() {
    let f = Fixture::new();
    let (_ocr, rx) = f.start(Duration::ZERO);
    let id = f.image(100, 1_000);
    let formats: i64 = f
        .store
        .clip_formats(id)
        .unwrap()
        .iter()
        .map(|f| f.data.len() as i64)
        .sum();

    assert_eq!(next(&rx), id);
    assert_eq!(
        f.text_of(id),
        (Some("image 100".into()), Some(IMAGE_TEXT_READER))
    );
    let clip = f.store.get_clip(id).unwrap().unwrap();
    assert_eq!(clip.size_bytes, formats + "image 100".len() as i64);
    assert_eq!(f.store.images_without_text().unwrap(), 0);
}

#[test]
fn writing_text_doesnt_give_the_clip_a_new_sync_seq() {
    let f = Fixture::new();
    let id = f.image(100, 1_000);
    let seq = f.store.sync_seq().unwrap();
    assert!(
        f.store
            .write_image_text(id, Some("Invoice"), IMAGE_TEXT_READER)
            .unwrap()
    );
    assert_eq!(f.store.sync_seq().unwrap(), seq);
    f.store.set_image_text_enabled(false).unwrap();
    assert_eq!(f.store.sync_seq().unwrap(), seq);
}

#[test]
fn the_backlog_is_read_newest_first() {
    let f = Fixture::new();
    let old = f.image(30, 1_000);
    let newest = f.image(50, 3_000);
    let middle = f.image(40, 2_000);
    assert_eq!(f.store.images_without_text().unwrap(), 3);

    let (_ocr, rx) = f.start(Duration::ZERO);
    assert_eq!([next(&rx), next(&rx), next(&rx)], [newest, middle, old]);
    assert_eq!(f.widths_read(), [50, 40, 30]);
}

#[test]
fn new_images_go_before_the_backlog_and_skip_its_pause() {
    let f = Fixture::new();
    f.image(30, 1_000);
    let second = f.image(40, 2_000);
    let (_ocr, rx) = f.start(Duration::from_secs(60));
    assert_eq!(next(&rx), second);

    // The backlog now waits a minute. A new image doesn't.
    let new = f.image(100, now_ms());
    assert_eq!(next(&rx), new);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    assert_eq!(f.widths_read(), [40, 100]);
}

#[test]
fn saving_power_pauses_the_backlog_but_not_new_images() {
    let f = Fixture::new();
    f.power.0.store(true, Ordering::SeqCst);
    f.image(30, 1_000);
    let (_ocr, rx) = f.start(Duration::ZERO);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());

    let new = f.image(100, now_ms());
    assert_eq!(next(&rx), new);
    assert_eq!(f.widths_read(), [100]);
    assert_eq!(f.store.images_without_text().unwrap(), 1);
}

#[test]
fn turning_it_off_clears_the_text_and_stops_reading_until_it_is_on_again() {
    let f = Fixture::new();
    let id = f.image(100, 1_000);
    // Taken before reading starts, so it can't include the text.
    let size = f.store.get_clip(id).unwrap().unwrap().size_bytes;
    let (ocr, rx) = f.start(Duration::ZERO);
    assert_eq!(next(&rx), id);

    f.store.set_image_text_enabled(false).unwrap();
    assert!(!f.store.image_text_enabled().unwrap());
    assert_eq!(f.text_of(id), (None, None));
    assert_eq!(f.store.get_clip(id).unwrap().unwrap().size_bytes, size);
    let off = f.image(200, 2_000);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    // Text read just before it was turned off isn't written either.
    assert!(
        !f.store
            .write_image_text(off, Some("late"), IMAGE_TEXT_READER)
            .unwrap()
    );

    f.store.set_image_text_enabled(true).unwrap();
    ocr.resume();
    let mut read = [next(&rx), next(&rx)];
    read.sort();
    assert_eq!(read, [id, off]);
    assert_eq!(f.text_of(id).0.as_deref(), Some("image 100"));
}

#[test]
fn without_a_recognizer_images_wait_until_a_resume_finds_one() {
    let f = Fixture::new();
    f.recognizer.missing.store(true, Ordering::SeqCst);
    let old = f.image(100, 1_000);
    let (ocr, rx) = f.start(Duration::ZERO);
    let new = f.image(200, 2_000);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    assert!(!ocr.readable());
    assert_eq!(f.text_of(new), (None, None));

    // A language with text recognition was installed.
    f.recognizer.missing.store(false, Ordering::SeqCst);
    ocr.resume();
    assert_eq!([next(&rx), next(&rx)], [new, old]);
    assert!(ocr.readable());
}

#[test]
fn an_image_that_cant_be_read_is_marked_and_not_tried_again() {
    let mut f = Fixture::new();
    f.recognizer.fail = vec![30];
    let failed = f.image(30, 2_000);
    let tiny = f.image(10, 1_000);
    let (ocr, rx) = f.start(Duration::ZERO);
    let mut read = [next(&rx), next(&rx)];
    read.sort();
    assert_eq!(read, [failed, tiny]);
    // Too small to hold text: never handed to the recognizer.
    assert_eq!(f.widths_read(), [30]);
    for id in [failed, tiny] {
        assert_eq!(f.text_of(id), (None, Some(IMAGE_TEXT_READER)));
    }

    // Nothing is left for the next launch.
    drop(ocr);
    let (_ocr, rx) = f.start(Duration::ZERO);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    assert_eq!(f.widths_read(), [30]);
}

#[test]
fn copying_an_image_again_keeps_its_text_and_its_size() {
    let f = Fixture::new();
    let (_ocr, rx) = f.start(Duration::ZERO);
    let id = f.image(100, 1_000);
    assert_eq!(next(&rx), id);
    let size = f.store.get_clip(id).unwrap().unwrap().size_bytes;

    assert_eq!(f.image(100, 2_000), id);
    let clip = f.store.get_clip(id).unwrap().unwrap();
    assert_eq!(clip.plain_text.as_deref(), Some("image 100"));
    assert_eq!(clip.ocr_version, Some(IMAGE_TEXT_READER));
    assert_eq!(clip.size_bytes, size);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_deleted_image_or_another_kind_is_never_written() {
    let f = Fixture::new();
    let id = f.image(100, 1_000);
    f.store.delete_clips(&[id]).unwrap();
    assert!(
        !f.store
            .write_image_text(id, Some("gone"), IMAGE_TEXT_READER)
            .unwrap()
    );

    let text = f
        .store
        .upsert_clip_at(
            NewClip {
                kind: ClipKind::Text,
                plain_text: Some("hello".into()),
                content_hash: content_hash(ClipKind::Text, b"hello"),
                source_app_name: None,
                source_app_id: None,
                metadata: None,
                platform: Platform::MacOs,
                formats: vec![],
                local_only: false,
            },
            1_000,
        )
        .unwrap()
        .id;
    assert!(
        !f.store
            .write_image_text(text, Some("nope"), IMAGE_TEXT_READER)
            .unwrap()
    );
    assert_eq!(f.text_of(text), (Some("hello".into()), None));
}

/// The real thing: Vision reads a copied screenshot, and search finds it by its text.
#[cfg(target_os = "macos")]
#[test]
fn a_copied_screenshot_is_found_by_the_text_in_it() {
    use kopi_core::search::{SearchQuery, search};
    use kopi_platform::macos::VisionRecognizer;

    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(dir.path()).unwrap());
    let (tx, rx) = mpsc::channel();
    let _ocr = Ocr::spawn(
        Arc::clone(&store),
        Box::new(VisionRecognizer),
        Box::new(FakePower::default()),
        Duration::ZERO,
        Box::new(move |id| {
            let _ = tx.send(id);
        }),
    )
    .unwrap();

    let png = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kopi-platform/tests/fixtures/invoice.png"
    ))
    .unwrap();
    let id = store
        .upsert_clip(NewClip {
            kind: ClipKind::Image,
            plain_text: None,
            content_hash: content_hash(ClipKind::Image, &png),
            source_app_name: None,
            source_app_id: None,
            metadata: None,
            platform: Platform::MacOs,
            formats: vec![NewFormat {
                item_index: 0,
                format: "public.png".into(),
                data: png,
            }],
            local_only: false,
        })
        .unwrap()
        .id;
    // Vision's first request in a process sometimes takes about a minute.
    assert_eq!(rx.recv_timeout(Duration::from_secs(120)).unwrap(), id);

    let page = search(
        &store,
        &SearchQuery {
            text: "Invoice 4821".into(),
            ..SearchQuery::default()
        },
    )
    .unwrap();
    assert_eq!(page.hits.len(), 1);
    assert_eq!(page.hits[0].id, id);
    assert!(page.hits[0].snippet.iter().any(|s| s.matched));
}
