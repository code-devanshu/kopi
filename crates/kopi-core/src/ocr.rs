//! Reads the text in image clips on a background thread, so search can find them. See
//! `docs/OCR_DESIGN.md`.
//!
//! New images are read as they arrive, from a copy or from sync. Images already in the history,
//! the backlog, are read newest first, one every [`BACKLOG_PAUSE`], and not while the computer
//! is saving power. Nothing is read while the setting is off.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use image::DynamicImage;
use kopi_platform::{PowerMode, RgbaImage, TextRecognizer, lower_thread_priority};

use crate::storage::{ClipFormat, IMAGE_TEXT_READER, SourceFormat, Store, decode, truncate_text};

/// How long the backlog waits after each image, so it takes one core part of the time.
pub const BACKLOG_PAUSE: Duration = Duration::from_millis(250);

/// How often a backlog paused for power saving checks whether it's over.
const POWER_CHECK: Duration = Duration::from_secs(60);

/// How long the backlog waits after the store failed, so a broken database isn't hammered.
const ERROR_PAUSE: Duration = Duration::from_secs(30);

/// Images narrower or shorter than this hold no text worth reading.
const MIN_EDGE: u32 = 16;

/// Called on the reading thread after an image's text is written, with the clip's id. It's
/// called for an image with no text too, which matters to a preview showing "Looking for text".
pub type OcrListener = Box<dyn Fn(i64) + Send + 'static>;

enum Message {
    /// An image clip was inserted.
    Added(i64),
    /// The setting was turned on.
    Resume,
    Stop,
}

/// Owns the reading thread. Dropping it stops the thread once the image being read is done.
pub struct Ocr {
    tx: Sender<Message>,
    readable: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Ocr {
    /// Starts the thread and has `store` tell it about new images. `pause` is
    /// [`BACKLOG_PAUSE`] outside tests.
    pub fn spawn(
        store: Arc<Store>,
        recognizer: Box<dyn TextRecognizer>,
        power: Box<dyn PowerMode>,
        pause: Duration,
        listener: OcrListener,
    ) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let readable = Arc::new(AtomicBool::new(false));
        let added = tx.clone();
        store.set_image_listener(Box::new(move |id| {
            // The thread only stops when the Ocr is dropped. Then there's no one to tell.
            let _ = added.send(Message::Added(id));
        }));
        let worker_readable = Arc::clone(&readable);
        let worker = thread::Builder::new()
            .name("kopi-ocr".into())
            .spawn(move || {
                lower_thread_priority();
                Worker {
                    store,
                    recognizer,
                    power,
                    pause,
                    listener,
                    readable: worker_readable,
                    rx,
                    new: VecDeque::new(),
                }
                .run();
            })?;
        Ok(Self {
            tx,
            readable,
            worker: Some(worker),
        })
    }

    /// The setting was turned on, or a recognizer may have become available: start on the
    /// images in the history.
    pub fn resume(&self) {
        let _ = self.tx.send(Message::Resume);
    }

    /// Whether the recognizer could run when the thread last looked, so a preview doesn't wait
    /// for text that won't come. False until it first looks, and while the setting is off.
    pub fn readable(&self) -> bool {
        self.readable.load(Ordering::Relaxed)
    }
}

impl Drop for Ocr {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct Worker {
    store: Arc<Store>,
    recognizer: Box<dyn TextRecognizer>,
    power: Box<dyn PowerMode>,
    pause: Duration,
    listener: OcrListener,
    readable: Arc<AtomicBool>,
    rx: Receiver<Message>,
    /// Images that just arrived. They go before the backlog, without its pause.
    new: VecDeque<i64>,
}

/// What the worker does after a step.
enum Next {
    Continue,
    /// Waits for a message, or until the time is up.
    Wait(Duration),
    /// Waits for a message.
    Sleep,
}

impl Worker {
    fn run(mut self) {
        // When the backlog may take its next image.
        let mut backlog_at = Instant::now();
        loop {
            loop {
                match self.rx.try_recv() {
                    Ok(message) => {
                        if !self.take(message) {
                            return;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            let message = match self.step(&mut backlog_at) {
                Next::Continue => continue,
                Next::Wait(time) => match self.rx.recv_timeout(time) {
                    Ok(message) => message,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => return,
                },
                Next::Sleep => match self.rx.recv() {
                    Ok(message) => message,
                    Err(_) => return,
                },
            };
            if !self.take(message) {
                return;
            }
        }
    }

    /// Queues what a message asks for. False means stop.
    fn take(&mut self, message: Message) -> bool {
        match message {
            Message::Added(id) => self.new.push_back(id),
            Message::Resume => {}
            Message::Stop => return false,
        }
        true
    }

    /// Reads one image if there's one to read now, and says what to do next.
    fn step(&mut self, backlog_at: &mut Instant) -> Next {
        match self.store.image_text_enabled() {
            Ok(true) => {}
            // Turning it on again reads everything as backlog.
            Ok(false) => {
                self.readable.store(false, Ordering::Relaxed);
                self.new.clear();
                return Next::Sleep;
            }
            Err(e) => {
                eprintln!("kopi: couldn't read the image text setting: {e}");
                return Next::Wait(ERROR_PAUSE);
            }
        }
        // On Windows there's none until a language with text recognition is installed. The
        // images wait as backlog, and a resume looks again.
        let available = self.recognizer.available();
        self.readable.store(available, Ordering::Relaxed);
        if !available {
            self.new.clear();
            return Next::Sleep;
        }
        if let Some(id) = self.new.pop_front() {
            self.read(id);
            return Next::Continue;
        }
        let now = Instant::now();
        if now < *backlog_at {
            return Next::Wait(*backlog_at - now);
        }
        if self.power.saving_power() {
            return Next::Wait(POWER_CHECK);
        }
        match self.store.next_image_without_text() {
            Ok(Some(id)) => {
                let pause = if self.read(id) {
                    self.pause
                } else {
                    ERROR_PAUSE
                };
                *backlog_at = Instant::now() + pause;
                Next::Continue
            }
            Ok(None) => Next::Sleep,
            Err(e) => {
                eprintln!("kopi: couldn't find images to read: {e}");
                *backlog_at = now + ERROR_PAUSE;
                Next::Continue
            }
        }
    }

    /// Reads image `id`, if it's still waiting, and writes its text. An image that can't be read
    /// is written with no text, so it isn't tried again. False if the store failed. Logs name
    /// the clip, never its text.
    fn read(&self, id: i64) -> bool {
        let formats = match self.store.unread_image(id) {
            Ok(Some(formats)) => formats,
            Ok(None) => return true,
            Err(e) => {
                eprintln!("kopi: couldn't load image {id} to read its text: {e}");
                return false;
            }
        };
        let text = text_in(self.recognizer.as_ref(), &formats).unwrap_or_else(|e| {
            eprintln!("kopi: couldn't read the text in image {id}: {e}");
            None
        });
        match self
            .store
            .write_image_text(id, text.as_deref(), IMAGE_TEXT_READER)
        {
            Ok(true) => {
                (self.listener)(id);
                true
            }
            // Deleted, read already, or the setting was turned off meanwhile.
            Ok(false) => true,
            Err(e) => {
                eprintln!("kopi: couldn't save the text read from image {id}: {e}");
                false
            }
        }
    }
}

/// The text in an image clip, from its best format: PNG, then TIFF, then DIB. None when there's
/// none.
fn text_in(
    recognizer: &dyn TextRecognizer,
    formats: &[ClipFormat],
) -> Result<Option<String>, String> {
    let (format, data) = formats
        .iter()
        .filter_map(|f| Some((SourceFormat::from_format_name(&f.format)?, &f.data)))
        .min_by_key(|(format, _)| *format as u8)
        .ok_or("it has no format Kopi can decode")?;
    let image = decode(data, format).map_err(|e| e.to_string())?;
    if image.width() < MIN_EDGE || image.height() < MIN_EDGE {
        return Ok(None);
    }
    let image = on_white(fit(image, recognizer.max_dimension()));
    let lines = recognizer.recognize(&image).map_err(|e| e.to_string())?;
    Ok(tidy(&lines))
}

/// Scales `image` down to fit `max` pixels on its longest edge, keeping its shape. This
/// averages 8-bit pixels: `resize`'s float buffers took 190 MB more for a 5K screenshot, for
/// the same text.
fn fit(image: DynamicImage, max: u32) -> DynamicImage {
    if image.width() <= max && image.height() <= max {
        return image;
    }
    image.thumbnail(max, max)
}

/// The image's pixels over white, so text on a transparent background isn't read as being on
/// black.
fn on_white(image: DynamicImage) -> RgbaImage {
    let mut rgba = image.into_rgba8();
    for pixel in rgba.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        if alpha < 255 {
            for channel in &mut pixel.0[..3] {
                *channel = ((u32::from(*channel) * alpha + 255 * (255 - alpha)) / 255) as u8;
            }
            pixel[3] = 255;
        }
    }
    RgbaImage {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    }
}

/// The recognized lines as one text: trimmed, without empty lines, joined with `\n`, and cut
/// to 64 KiB. None when nothing's left.
pub(crate) fn tidy(lines: &[String]) -> Option<String> {
    let text = lines
        .iter()
        .map(|line| close_cjk_gaps(line.trim()))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then(|| truncate_text(text).0)
}

/// Removes the spaces between two Chinese or Japanese characters, which Windows puts between
/// every character, so a search for a word finds it.
fn close_cjk_gaps(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut spaces = 0;
    let mut last = None;
    for c in line.chars() {
        if c == ' ' {
            spaces += 1;
            continue;
        }
        if spaces > 0 && !(last.is_some_and(is_cjk) && is_cjk(c)) {
            out.extend(std::iter::repeat_n(' ', spaces));
        }
        spaces = 0;
        out.push(c);
        last = Some(c);
    }
    out
}

/// Han, kana and CJK punctuation. Not Hangul: Korean puts spaces between words.
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3000..=0x303F // CJK symbols and punctuation
        | 0x3040..=0x30FF // Hiragana and Katakana
        | 0x31F0..=0x31FF // Katakana phonetic extensions
        | 0x3400..=0x4DBF // CJK extension A
        | 0x4E00..=0x9FFF // CJK unified ideographs
        | 0xF900..=0xFAFF // CJK compatibility ideographs
        | 0xFF00..=0xFFEF // Halfwidth and fullwidth forms
        | 0x20000..=0x2FA1F // CJK extensions B to F, and compatibility supplement
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MAX_TEXT_INDEX_BYTES;

    fn lines(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|&l| l.to_owned()).collect()
    }

    #[test]
    fn tidy_trims_drops_empty_lines_and_joins() {
        assert_eq!(
            tidy(&lines(&["  Invoice 42 ", "", "   ", "Total\tdue"])).as_deref(),
            Some("Invoice 42\nTotal\tdue")
        );
        assert_eq!(tidy(&lines(&[" ", ""])), None);
        assert_eq!(tidy(&[]), None);
    }

    #[test]
    fn tidy_closes_gaps_between_chinese_and_japanese_characters_only() {
        assert_eq!(
            tidy(&lines(&[
                "東 京 タ ワ ー",
                "价 格 ： 42 元",
                "안녕 하세요",
                "a  b"
            ]))
            .as_deref(),
            Some("東京タワー\n价格： 42 元\n안녕 하세요\na  b")
        );
    }

    #[test]
    fn tidy_cuts_to_the_index_limit_at_a_char_boundary() {
        let long = "é".repeat(MAX_TEXT_INDEX_BYTES);
        let text = tidy(&[long]).unwrap();
        assert!(text.len() <= MAX_TEXT_INDEX_BYTES);
        assert!(text.len() > MAX_TEXT_INDEX_BYTES - 2);
    }

    #[test]
    fn transparent_pixels_go_on_white() {
        let mut image = image::RgbaImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
        image.put_pixel(1, 0, image::Rgba([0, 0, 0, 255]));
        let out = on_white(DynamicImage::ImageRgba8(image));
        assert_eq!(out.pixels, [255, 255, 255, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn fit_scales_the_longest_edge_down_keeping_the_shape() {
        let image = DynamicImage::new_rgba8(8000, 2000);
        let fitted = fit(image, 4096);
        assert_eq!((fitted.width(), fitted.height()), (4096, 1024));
        let small = fit(DynamicImage::new_rgba8(100, 50), 4096);
        assert_eq!((small.width(), small.height()), (100, 50));
    }
}
