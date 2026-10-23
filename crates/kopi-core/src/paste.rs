//! Puts a clip back on the clipboard and pastes it into the app in front. See
//! `docs/decisions/0004-paste-simulation-approach.md`.
//!
//! 1. Write the clip: every format it was copied with, or only its plain text. The write is
//!    marked as Kopi's own, so the watcher doesn't capture it again.
//! 2. Move the clip to the top: `last_used_at` and `use_count`.
//! 3. If Kopi may post keys, hide the panel and post the paste shortcut. Otherwise the clip
//!    stays on the clipboard for the user to paste, and the system is asked for the permission,
//!    once per run.
//!
//! A snippet pastes the same way, as plain text the panel filled in. It isn't a clip, so
//! nothing moves to the top, and the marked write keeps it out of the history.
//!
//! Formats are macOS names (UTIs) on every OS. The Windows clipboard translates them.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use kopi_platform::{
    KeyError, KeySender, Pasteboard, PasteboardError, Permission, Permissions, RawFormat, uti,
};
use thiserror::Error;

use crate::classifier::file_path;
use crate::storage::{Clip, ClipFormat, ClipKind, Platform, StorageError, Store, now_ms};

/// What to put on the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
    /// Every stored format, so rich text keeps its formatting and files paste as files.
    All,
    /// Only the plain text. A clip with no text, such as an image, is written whole.
    PlainText,
}

/// How a paste ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pasted {
    /// The shortcut was posted to the app in front.
    Pasted,
    /// Kopi may not post keys, so the clip is only on the clipboard.
    Copied,
}

/// After this many characters past `{cursor}`, the cursor stays at the end: posting that many
/// arrow keys is slow, and the user sees it.
pub const MAX_CURSOR_MOVE: usize = 1000;

/// How long the app gets to take the paste before the arrow keys arrive. Some apps, such as
/// Electron ones, read the clipboard a moment after ⌘V.
const PASTE_SETTLE: Duration = Duration::from_millis(60);

#[derive(Debug, Error)]
pub enum PasteError {
    #[error("no clip {0}")]
    NotFound(i64),
    #[error("no snippet {0}")]
    NoSnippet(i64),
    /// The clip isn't an image, or no text has been read from it.
    #[error("no text in clip {0}")]
    NoImageText(i64),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Pasteboard(#[from] PasteboardError),
    #[error(transparent)]
    Keys(#[from] KeyError),
}

pub struct PasteEngine {
    pasteboard: Mutex<Box<dyn Pasteboard>>,
    keys: Box<dyn KeySender>,
    permissions: Box<dyn Permissions>,
    /// Marks Kopi's writes. The watcher skips a change with this in its source marker.
    own_bundle_id: String,
    /// Whether the system was asked for Accessibility during this run.
    asked: AtomicBool,
}

impl PasteEngine {
    pub fn new(
        pasteboard: Box<dyn Pasteboard>,
        keys: Box<dyn KeySender>,
        permissions: Box<dyn Permissions>,
        own_bundle_id: impl Into<String>,
    ) -> Self {
        Self {
            pasteboard: Mutex::new(pasteboard),
            keys,
            permissions,
            own_bundle_id: own_bundle_id.into(),
            asked: AtomicBool::new(false),
        }
    }

    /// Puts clip `id` on the clipboard and moves it to the top.
    pub fn copy(&self, store: &Store, id: i64, content: Content) -> Result<(), PasteError> {
        let clip = store.get_clip(id)?.ok_or(PasteError::NotFound(id))?;
        let formats = clipboard_formats(&clip, &store.clip_formats(id)?, content);
        self.write(&formats)?;
        store.mark_used(id, now_ms())?;
        Ok(())
    }

    /// Copies clip `id`, then calls `hide`, which must have hidden the panel when it returns,
    /// and posts the paste shortcut. Without the Accessibility permission the clip is only
    /// copied, and `hide` isn't called.
    pub fn paste(
        &self,
        store: &Store,
        id: i64,
        content: Content,
        hide: impl FnOnce(),
    ) -> Result<Pasted, PasteError> {
        self.copy(store, id, content)?;
        self.post_paste(hide, 0)
    }

    /// Puts the text read from image clip `id` on the clipboard as plain text, marked as Kopi's,
    /// so it doesn't become a clip. The image doesn't move in the history.
    pub fn copy_image_text(&self, store: &Store, id: i64) -> Result<(), PasteError> {
        let clip = store.get_clip(id)?.ok_or(PasteError::NotFound(id))?;
        let text = match (clip.kind, clip.plain_text) {
            (ClipKind::Image, Some(text)) if !text.is_empty() => text,
            _ => return Err(PasteError::NoImageText(id)),
        };
        self.write(&[text_format(text.into_bytes())])?;
        Ok(())
    }

    /// Puts snippet `id`'s filled-in `text` on the clipboard as plain text, and counts the use.
    pub fn copy_snippet(&self, store: &Store, id: i64, text: &str) -> Result<(), PasteError> {
        if store.snippet(id)?.is_none() {
            return Err(PasteError::NoSnippet(id));
        }
        self.write(&[text_format(text.as_bytes().to_vec())])?;
        store.mark_snippet_used(id, now_ms())?;
        Ok(())
    }

    /// Copies a snippet's filled-in `text`, then pastes it as [`paste`](Self::paste) does a
    /// clip. `cursor_back` is how many characters the cursor moves left after the paste, for
    /// `{cursor}`. Past [`MAX_CURSOR_MOVE`] it stays at the end.
    pub fn paste_snippet(
        &self,
        store: &Store,
        id: i64,
        text: &str,
        cursor_back: usize,
        hide: impl FnOnce(),
    ) -> Result<Pasted, PasteError> {
        self.copy_snippet(store, id, text)?;
        let cursor_back = if cursor_back > MAX_CURSOR_MOVE {
            0
        } else {
            cursor_back
        };
        self.post_paste(hide, cursor_back)
    }

    fn write(&self, formats: &[RawFormat]) -> Result<(), PasteError> {
        self.pasteboard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .write(formats, Some(&self.own_bundle_id))?;
        Ok(())
    }

    /// Hides the panel and posts the paste shortcut, then `cursor_back` Left arrows. Without
    /// the Accessibility permission, asks for it once per run and leaves the panel up.
    fn post_paste(&self, hide: impl FnOnce(), cursor_back: usize) -> Result<Pasted, PasteError> {
        // Posted keys are dropped without the permission, and nothing reports it.
        if !self.permissions.is_granted(Permission::Accessibility) {
            if !self.asked.swap(true, Ordering::SeqCst) {
                self.permissions.request(Permission::Accessibility);
            }
            return Ok(Pasted::Copied);
        }
        // The panel is key while it shows, so the shortcut would go to it.
        hide();
        self.keys.send_paste()?;
        if cursor_back > 0 {
            thread::sleep(PASTE_SETTLE);
            self.keys.send_left(cursor_back)?;
        }
        Ok(Pasted::Pasted)
    }
}

/// The full text of the newest clip with text, for a snippet's `{clipboard}`: what ⌥Return
/// would paste. None when the history has no text.
pub fn newest_text(store: &Store) -> Result<Option<String>, PasteError> {
    let Some(id) = store.newest_text_clip()? else {
        return Ok(None);
    };
    let Some(clip) = store.get_clip(id)? else {
        return Ok(None);
    };
    let formats = clipboard_formats(&clip, &store.clip_formats(id)?, Content::PlainText);
    Ok(formats
        .into_iter()
        .find(|f| f.format == uti::PLAIN_TEXT)
        .map(|f| String::from_utf8_lossy(&f.data).into_owned()))
}

/// The formats to write for `clip`, by pasteboard item, without the source marker.
pub fn clipboard_formats(clip: &Clip, stored: &[ClipFormat], content: Content) -> Vec<RawFormat> {
    let stored: Vec<&ClipFormat> = stored
        .iter()
        .filter(|f| f.platform == Platform::MacOs)
        .collect();
    let stored_text: Vec<&[u8]> = stored
        .iter()
        .filter(|f| f.format == uti::PLAIN_TEXT)
        .map(|f| &f.data[..])
        .collect();

    if content == Content::PlainText
        && let Some(text) = plain_text(clip, &stored, &stored_text)
    {
        return vec![text_format(text)];
    }

    let mut out: Vec<RawFormat> = stored
        .iter()
        .map(|f| RawFormat {
            item_index: f.item_index,
            format: f.format.clone(),
            data: f.data.clone(),
        })
        .collect();
    // The store drops a plain-text format that's a copy of `plain_text`, so put it back. A
    // file clip's text is its paths, and an image's is what was read from it, which Kopi made
    // up, so neither is added.
    if stored_text.is_empty()
        && !matches!(clip.kind, ClipKind::File | ClipKind::Image)
        && let Some(text) = &clip.plain_text
    {
        let first = out.iter().map(|f| f.item_index).min().unwrap_or(0);
        out.insert(
            0,
            RawFormat {
                item_index: first,
                ..text_format(text.as_bytes().to_vec())
            },
        );
    }
    out
}

/// The text ⌥Return pastes: `plain_text`, which for a link is the URL, or a file clip's full
/// paths. When `plain_text` was cut for the index, the full text comes from the stored formats.
/// None for an image, which pastes as it was copied, whatever text was read from it.
fn plain_text(clip: &Clip, stored: &[&ClipFormat], stored_text: &[&[u8]]) -> Option<Vec<u8>> {
    if clip.kind == ClipKind::Image {
        return None;
    }
    if clip.kind == ClipKind::File {
        let paths: Vec<String> = stored
            .iter()
            .filter(|f| f.format == uti::FILE_URL)
            .filter_map(|f| file_path(&f.data))
            .collect();
        if !paths.is_empty() {
            return Some(paths.join("\n").into_bytes());
        }
    }
    let text = clip.plain_text.as_deref()?;
    let full = stored_text.join(&b"\n"[..]);
    if full.len() > text.len() && full.starts_with(text.as_bytes()) {
        return Some(full);
    }
    Some(text.as_bytes().to_vec())
}

fn text_format(data: Vec<u8>) -> RawFormat {
    RawFormat {
        item_index: 0,
        format: uti::PLAIN_TEXT.to_owned(),
        data,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kopi_platform::{PasteboardAccess, PasteboardTypes};

    use super::*;
    use crate::classifier::classify;
    use crate::storage::MAX_TEXT_INDEX_BYTES;

    const KOPI: &str = "in.devanshuverma.kopi";

    #[derive(Default)]
    struct Log {
        writes: Vec<(Vec<RawFormat>, Option<String>)>,
        events: Vec<&'static str>,
        lefts: Vec<usize>,
        granted: bool,
    }

    #[derive(Clone, Default)]
    struct Fake(Arc<Mutex<Log>>);

    impl Fake {
        fn log(&self) -> std::sync::MutexGuard<'_, Log> {
            self.0.lock().unwrap()
        }
    }

    impl Pasteboard for Fake {
        fn change_count(&self) -> Result<u64, PasteboardError> {
            Ok(0)
        }
        fn types(&self) -> Result<PasteboardTypes, PasteboardError> {
            Ok(PasteboardTypes::default())
        }
        fn read(
            &self,
            _: u64,
            _: &[Vec<String>],
            _: u64,
        ) -> Result<Vec<RawFormat>, PasteboardError> {
            Ok(Vec::new())
        }
        fn access(&self) -> Result<PasteboardAccess, PasteboardError> {
            Ok(PasteboardAccess::AlwaysAllow)
        }
        fn write(
            &self,
            formats: &[RawFormat],
            source: Option<&str>,
        ) -> Result<u64, PasteboardError> {
            let mut log = self.log();
            log.writes
                .push((formats.to_vec(), source.map(str::to_owned)));
            log.events.push("write");
            Ok(1)
        }
    }

    impl KeySender for Fake {
        fn send_paste(&self) -> Result<(), KeyError> {
            self.log().events.push("send_paste");
            Ok(())
        }
        fn send_left(&self, count: usize) -> Result<(), KeyError> {
            let mut log = self.log();
            log.events.push("send_left");
            log.lefts.push(count);
            Ok(())
        }
    }

    impl Permissions for Fake {
        fn is_granted(&self, _: Permission) -> bool {
            self.log().granted
        }
        fn request(&self, _: Permission) {
            self.log().events.push("request");
        }
        fn open_settings(&self, _: Permission) {}
    }

    struct Harness {
        _dir: tempfile::TempDir,
        store: Store,
        fake: Fake,
        engine: PasteEngine,
    }

    fn harness(granted: bool) -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let fake = Fake::default();
        fake.log().granted = granted;
        let engine = PasteEngine::new(
            Box::new(fake.clone()),
            Box::new(fake.clone()),
            Box::new(fake.clone()),
            KOPI,
        );
        Harness {
            _dir: dir,
            store,
            fake,
            engine,
        }
    }

    fn raw(item_index: u32, format: &str, data: &[u8]) -> RawFormat {
        RawFormat {
            item_index,
            format: format.to_owned(),
            data: data.to_vec(),
        }
    }

    fn save(store: &Store, formats: Vec<RawFormat>) -> i64 {
        let clip = classify(formats).unwrap().into_clip(None);
        store.upsert_clip_at(clip, 1_000).unwrap().id
    }

    fn written(h: &Harness, id: i64, content: Content) -> Vec<(u32, String, Vec<u8>)> {
        let clip = h.store.get_clip(id).unwrap().unwrap();
        let formats = h.store.clip_formats(id).unwrap();
        clipboard_formats(&clip, &formats, content)
            .into_iter()
            .map(|f| (f.item_index, f.format, f.data))
            .collect()
    }

    fn f(item: u32, format: &str, data: &[u8]) -> (u32, String, Vec<u8>) {
        (item, format.to_owned(), data.to_vec())
    }

    #[test]
    fn plain_text_comes_back_though_the_store_dropped_it() {
        let h = harness(true);
        let id = save(&h.store, vec![raw(0, uti::PLAIN_TEXT, b"Hello")]);
        assert_eq!(h.store.clip_formats(id).unwrap(), []);
        assert_eq!(
            written(&h, id, Content::All),
            [f(0, uti::PLAIN_TEXT, b"Hello")]
        );
    }

    #[test]
    fn rich_text_keeps_every_format_or_only_its_text() {
        let h = harness(true);
        let id = save(
            &h.store,
            vec![
                raw(0, uti::PLAIN_TEXT, b"Bold move"),
                raw(0, uti::HTML, b"<b>Bold</b> move"),
                raw(0, uti::RTF, b"{\\rtf1 Bold move}"),
            ],
        );
        let all = written(&h, id, Content::All);
        assert_eq!(all[0], f(0, uti::PLAIN_TEXT, b"Bold move"));
        assert!(all.contains(&f(0, uti::HTML, b"<b>Bold</b> move")));
        assert!(all.contains(&f(0, uti::RTF, b"{\\rtf1 Bold move}")));
        assert_eq!(
            written(&h, id, Content::PlainText),
            [f(0, uti::PLAIN_TEXT, b"Bold move")]
        );
    }

    #[test]
    fn a_link_pastes_its_url_as_plain_text() {
        let h = harness(true);
        let id = save(
            &h.store,
            vec![
                raw(0, uti::URL, b"https://example.com/"),
                raw(0, uti::PLAIN_TEXT, b"Example Domain"),
            ],
        );
        assert!(written(&h, id, Content::All).contains(&f(0, uti::PLAIN_TEXT, b"Example Domain")));
        assert_eq!(
            written(&h, id, Content::PlainText),
            [f(0, uti::PLAIN_TEXT, b"https://example.com/")]
        );
    }

    #[test]
    fn files_keep_their_items() {
        let h = harness(true);
        let id = save(
            &h.store,
            vec![
                raw(0, uti::FILE_URL, b"file:///tmp/a.txt"),
                raw(1, uti::FILE_URL, b"file:///tmp/b.txt"),
            ],
        );
        assert_eq!(
            written(&h, id, Content::All),
            [
                f(0, uti::FILE_URL, b"file:///tmp/a.txt"),
                f(1, uti::FILE_URL, b"file:///tmp/b.txt"),
            ]
        );
        assert_eq!(
            written(&h, id, Content::PlainText),
            [f(0, uti::PLAIN_TEXT, b"/tmp/a.txt\n/tmp/b.txt")]
        );
    }

    #[test]
    fn plain_text_of_a_long_clip_is_the_full_text() {
        let h = harness(true);
        let long = "a".repeat(MAX_TEXT_INDEX_BYTES + 10);
        let id = save(&h.store, vec![raw(0, uti::PLAIN_TEXT, long.as_bytes())]);
        let text = written(&h, id, Content::PlainText);
        assert_eq!(text.len(), 1);
        assert_eq!(text[0].2.len(), long.len());
    }

    #[test]
    fn an_image_has_no_plain_text_so_it_is_written_whole() {
        let h = harness(true);
        let mut png = Vec::new();
        image::RgbaImage::new(2, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let id = save(&h.store, vec![raw(0, uti::PNG, &png)]);
        assert_eq!(written(&h, id, Content::PlainText), [f(0, uti::PNG, &png)]);

        // Text read from it is for search. It's never pasted, plain or not.
        assert!(
            h.store
                .write_image_text(id, Some("Invoice 42"), crate::storage::IMAGE_TEXT_READER)
                .unwrap()
        );
        for content in [Content::All, Content::PlainText] {
            assert_eq!(written(&h, id, content), [f(0, uti::PNG, &png)]);
        }
    }

    #[test]
    fn copy_image_text_writes_only_the_text_marked_and_leaves_the_image_in_place() {
        let h = harness(true);
        let mut png = Vec::new();
        image::RgbaImage::new(2, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let image = save(&h.store, vec![raw(0, uti::PNG, &png)]);
        let text = save(&h.store, vec![raw(0, uti::PLAIN_TEXT, b"Hello")]);
        assert!(matches!(
            h.engine.copy_image_text(&h.store, image),
            Err(PasteError::NoImageText(_))
        ));
        assert!(matches!(
            h.engine.copy_image_text(&h.store, text),
            Err(PasteError::NoImageText(_))
        ));

        h.store
            .write_image_text(image, Some("Invoice 42"), crate::storage::IMAGE_TEXT_READER)
            .unwrap();
        h.engine.copy_image_text(&h.store, image).unwrap();
        let log = h.fake.log();
        assert_eq!(log.writes.len(), 1);
        let (formats, marker) = &log.writes[0];
        assert_eq!(
            formats
                .iter()
                .map(|f| (f.format.as_str(), &f.data[..]))
                .collect::<Vec<_>>(),
            [(uti::PLAIN_TEXT, &b"Invoice 42"[..])]
        );
        assert_eq!(marker.as_deref(), Some(KOPI));
        let clip = h.store.get_clip(image).unwrap().unwrap();
        assert_eq!((clip.use_count, clip.last_used_at), (1, 1_000));
    }

    #[test]
    fn paste_writes_marked_moves_to_the_top_hides_then_posts() {
        let h = harness(true);
        let id = save(&h.store, vec![raw(0, uti::PLAIN_TEXT, b"Hello")]);
        let fake = h.fake.clone();
        let outcome = h
            .engine
            .paste(&h.store, id, Content::All, || {
                fake.log().events.push("hide")
            })
            .unwrap();
        assert_eq!(outcome, Pasted::Pasted);
        let log = h.fake.log();
        assert_eq!(log.events, ["write", "hide", "send_paste"]);
        assert_eq!(log.writes[0].1.as_deref(), Some(KOPI));
        let clip = h.store.get_clip(id).unwrap().unwrap();
        assert_eq!(clip.use_count, 2);
        assert!(clip.last_used_at > 1_000);
    }

    #[test]
    fn without_accessibility_it_copies_and_asks_once() {
        let h = harness(false);
        let id = save(&h.store, vec![raw(0, uti::PLAIN_TEXT, b"Hello")]);
        for _ in 0..2 {
            let outcome = h
                .engine
                .paste(&h.store, id, Content::All, || panic!("hid the panel"))
                .unwrap();
            assert_eq!(outcome, Pasted::Copied);
        }
        assert_eq!(h.fake.log().events, ["write", "request", "write"]);
        assert_eq!(h.store.get_clip(id).unwrap().unwrap().use_count, 3);
    }

    #[test]
    fn copy_never_posts() {
        let h = harness(true);
        let id = save(&h.store, vec![raw(0, uti::PLAIN_TEXT, b"Hello")]);
        h.engine.copy(&h.store, id, Content::PlainText).unwrap();
        assert_eq!(h.fake.log().events, ["write"]);
    }

    fn new_snippet(store: &Store) -> i64 {
        store
            .create_snippet(crate::storage::SnippetEdit {
                name: "Greeting",
                keyword: Some("hi"),
                body: "Hi {input:Name},{cursor}",
            })
            .unwrap()
            .id
    }

    #[test]
    fn a_snippet_pastes_its_text_marked_then_moves_the_cursor() {
        let h = harness(true);
        let id = new_snippet(&h.store);
        let fake = h.fake.clone();
        let outcome = h
            .engine
            .paste_snippet(&h.store, id, "<b></b>", 4, || {
                fake.log().events.push("hide")
            })
            .unwrap();
        assert_eq!(outcome, Pasted::Pasted);
        let log = h.fake.log();
        assert_eq!(log.events, ["write", "hide", "send_paste", "send_left"]);
        assert_eq!(log.lefts, [4]);
        assert_eq!(log.writes[0].0, [text_format(b"<b></b>".to_vec())]);
        assert_eq!(log.writes[0].1.as_deref(), Some(KOPI));
        drop(log);
        let snippet = h.store.snippet(id).unwrap().unwrap();
        assert_eq!(snippet.use_count, 1);
        assert!(snippet.last_used_at.is_some());
        // Not a clip.
        assert_eq!(h.store.newest_text_clip().unwrap(), None);
    }

    #[test]
    fn a_snippet_without_cursor_or_past_the_limit_leaves_the_cursor_at_the_end() {
        let h = harness(true);
        let id = new_snippet(&h.store);
        for back in [0, MAX_CURSOR_MOVE + 1] {
            h.engine
                .paste_snippet(&h.store, id, "text", back, || {})
                .unwrap();
        }
        h.engine
            .paste_snippet(&h.store, id, "text", MAX_CURSOR_MOVE, || {})
            .unwrap();
        assert_eq!(h.fake.log().lefts, [MAX_CURSOR_MOVE]);
    }

    #[test]
    fn a_snippet_without_accessibility_is_only_copied() {
        let h = harness(false);
        let id = new_snippet(&h.store);
        let outcome = h
            .engine
            .paste_snippet(&h.store, id, "Hi", 1, || panic!("hid the panel"))
            .unwrap();
        assert_eq!(outcome, Pasted::Copied);
        assert_eq!(h.fake.log().events, ["write", "request"]);
    }

    #[test]
    fn a_deleted_snippet_writes_nothing() {
        let h = harness(true);
        let err = h
            .engine
            .paste_snippet(&h.store, 42, "Hi", 0, || {})
            .unwrap_err();
        assert!(matches!(err, PasteError::NoSnippet(42)));
        assert!(h.fake.log().events.is_empty());
    }

    #[test]
    fn newest_text_is_the_newest_clip_with_text_in_full() {
        let h = harness(true);
        assert_eq!(newest_text(&h.store).unwrap(), None);
        let long = "b".repeat(MAX_TEXT_INDEX_BYTES + 10);
        let text = save(&h.store, vec![raw(0, uti::PLAIN_TEXT, long.as_bytes())]);
        let mut png = Vec::new();
        image::RgbaImage::new(2, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let image = save(&h.store, vec![raw(0, uti::PNG, &png)]);
        h.store.mark_used(text, 2_000).unwrap();
        h.store.mark_used(image, 3_000).unwrap();
        // The image is newer, but has no text.
        assert_eq!(newest_text(&h.store).unwrap(), Some(long));
        let link = save(
            &h.store,
            vec![
                raw(0, uti::URL, b"https://example.com/"),
                raw(0, uti::PLAIN_TEXT, b"Example Domain"),
            ],
        );
        h.store.mark_used(link, 4_000).unwrap();
        assert_eq!(
            newest_text(&h.store).unwrap().as_deref(),
            Some("https://example.com/")
        );
    }

    #[test]
    fn a_deleted_clip_writes_nothing() {
        let h = harness(true);
        let err = h
            .engine
            .paste(&h.store, 42, Content::All, || {})
            .unwrap_err();
        assert!(matches!(err, PasteError::NotFound(42)));
        assert!(h.fake.log().events.is_empty());
    }
}
