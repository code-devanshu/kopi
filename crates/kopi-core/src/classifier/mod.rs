//! Turns the raw formats of a copy into a clip: picks the kind, extracts the plain text and
//! the metadata, and computes the content hash that duplicates share. See `docs/DATA_MODEL.md`.
//!
//! The first kind that fits wins:
//!
//! 1. `file`: an item has a file URL.
//! 2. `color`: a color archive (`com.apple.cocoa.pasteboard.color`) that decodes.
//! 3. With plain text: `link` if the URL format or the whole text is a web address, `color`
//!    if the whole text is a CSS color, `code` if it reads as source code, `rich_text` if RTF,
//!    RTFD or HTML came with it, `text` otherwise.
//! 4. Without plain text: `image` (PNG, else TIFF), then `link` from the URL format, then
//!    `rich_text`.
//!
//! Each kind can only come from a format that
//! [`possible_kinds`](crate::privacy::possible_kinds) maps to it, so the privacy filter's
//! first stage never lets through a copy it would have known to skip.
//!
//! macOS formats only. Windows formats arrive with the Windows watcher.

mod code;
mod color;
mod link;

use std::io::Cursor;

use kopi_platform::{AppRef, RawFormat, uti};
use serde_json::{Value, json};

use crate::privacy::PrivacyFilter;
use crate::storage::{
    ClipKind, NewClip, NewFormat, Platform, StorageResult, Store, Upserted, truncate_text,
};
use crate::watcher::Capture;

/// The reason [`save`] gives for a copy with nothing it could make a clip of.
pub const UNRECOGNISED: &str = "unrecognised";

/// A classified copy. [`save`] runs it past the privacy filter and stores it.
#[derive(Debug, Clone, PartialEq)]
pub struct Classified {
    pub kind: ClipKind,
    /// Display and search text, cut to
    /// [`MAX_TEXT_INDEX_BYTES`](crate::storage::MAX_TEXT_INDEX_BYTES). The hash covers the
    /// full text, and the full text stays in `formats`.
    pub plain_text: Option<String>,
    /// BLAKE3 hex of the kind, a zero byte and the canonical payload.
    pub content_hash: String,
    /// `{w,h}` for images, `{rgba}` for colors, `{host}` for links, `{count,names}` for files
    /// and `{lang}` for code, when the language is clear.
    pub metadata: Option<Value>,
    /// Every format that was read, so paste can put back what was copied.
    pub formats: Vec<NewFormat>,
}

impl Classified {
    /// The total size of the formats, which the privacy filter checks against its limit.
    pub fn size_bytes(&self) -> u64 {
        self.formats.iter().map(|f| f.data.len() as u64).sum()
    }

    pub fn into_clip(self, source: Option<AppRef>) -> NewClip {
        let (source_app_name, source_app_id) =
            source.map_or((None, None), |a| (a.name, a.bundle_id));
        NewClip {
            kind: self.kind,
            plain_text: self.plain_text,
            content_hash: self.content_hash,
            source_app_name,
            source_app_id,
            metadata: self.metadata,
            platform: Platform::MacOs,
            formats: self.formats,
        }
    }
}

/// Classifies the formats of one copy. `None` if none of them makes a clip, such as a file URL
/// that isn't a path.
pub fn classify(formats: Vec<RawFormat>) -> Option<Classified> {
    let picked = pick(&formats)?;
    Some(Classified {
        kind: picked.kind,
        plain_text: picked.text.map(|text| truncate_text(text).0),
        content_hash: picked.hash,
        metadata: picked.metadata,
        formats: formats
            .into_iter()
            .map(|f| NewFormat {
                item_index: f.item_index,
                format: f.format,
                data: f.data,
            })
            .collect(),
    })
}

/// What [`save`] did with a copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// Stored. `clip.inserted` is `false` for a duplicate, which moved to the top.
    Stored { kind: ClipKind, clip: Upserted },
    /// Not stored: the privacy filter's reason, or [`UNRECOGNISED`].
    Dropped(&'static str),
}

/// Classifies a capture, runs the privacy filter's second stage on it, and stores it. Copying
/// something already in the history moves it to the top instead of adding it again.
pub fn save(store: &Store, filter: &PrivacyFilter, capture: Capture) -> StorageResult<Saved> {
    let Some(classified) = classify(capture.formats) else {
        return Ok(Saved::Dropped(UNRECOGNISED));
    };
    if let Err(reason) = filter.after_read(classified.kind, classified.size_bytes()) {
        return Ok(Saved::Dropped(reason));
    }
    let kind = classified.kind;
    let clip = store.upsert_clip(classified.into_clip(capture.source))?;
    Ok(Saved::Stored { kind, clip })
}

/// `BLAKE3(kind ‖ 0x00 ‖ payload)`, as hex.
pub fn content_hash(kind: ClipKind, payload: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(kind.as_str().as_bytes());
    hasher.update(&[0]);
    hasher.update(payload);
    hasher.finalize().to_hex().to_string()
}

struct Picked {
    kind: ClipKind,
    /// The full text, before it's cut.
    text: Option<String>,
    hash: String,
    metadata: Option<Value>,
}

impl Picked {
    /// A kind whose hash covers its plain text.
    fn text(kind: ClipKind, text: String, metadata: Option<Value>) -> Self {
        Self {
            kind,
            hash: content_hash(kind, text.as_bytes()),
            text: Some(text),
            metadata,
        }
    }
}

fn pick(formats: &[RawFormat]) -> Option<Picked> {
    let all = |format: &'static str| formats.iter().filter(move |f| f.format == format);
    let first = |format| all(format).next().map(|f| &f.data[..]);

    let paths: Vec<String> = all(uti::FILE_URL)
        .filter_map(|f| file_path(&f.data))
        .collect();
    if !paths.is_empty() {
        return Some(files(paths));
    }
    if let Some(rgba) = first(uti::COLOR).and_then(color::from_archive) {
        return Some(self::color(rgba));
    }

    let url_link = first(uti::URL).and_then(|data| link(decode(data).trim()));
    let rich = [uti::RTF, uti::RTFD, uti::HTML]
        .into_iter()
        .find_map(&first);
    let texts: Vec<String> = all(uti::PLAIN_TEXT).map(|f| decode(&f.data)).collect();
    let text = texts.join("\n");

    if !text.is_empty() {
        return Some(url_link.unwrap_or_else(|| from_text(text, rich.is_some())));
    }
    if let Some(data) = first(uti::PNG).or_else(|| first(uti::TIFF)) {
        return Some(image(data));
    }
    if url_link.is_some() {
        return url_link;
    }
    // Rich text with no plain-text version: rare, since NSAttributedString writes both.
    // There's no text to hash, so the rich data stands in.
    rich.map(|data| Picked {
        kind: ClipKind::RichText,
        text: None,
        hash: content_hash(ClipKind::RichText, data),
        metadata: None,
    })
}

fn from_text(text: String, rich: bool) -> Picked {
    let trimmed = text.trim();
    if let Some(link) = link(trimmed) {
        return link;
    }
    if let Some(rgba) = color::parse(trimmed) {
        return color(rgba);
    }
    if let Some(code) = code::detect(&text) {
        let metadata = code.lang.map(|lang| json!({ "lang": lang }));
        return Picked::text(ClipKind::Code, text, metadata);
    }
    let kind = if rich {
        ClipKind::RichText
    } else {
        ClipKind::Text
    };
    Picked::text(kind, text, None)
}

fn link(url: &str) -> Option<Picked> {
    let host = link::host(url)?;
    Some(Picked::text(
        ClipKind::Link,
        url.to_owned(),
        Some(json!({ "host": host })),
    ))
}

fn color(rgba: color::Rgba) -> Picked {
    Picked::text(
        ClipKind::Color,
        color::hex(rgba),
        Some(json!({ "rgba": rgba })),
    )
}

fn image(data: &[u8]) -> Picked {
    let size = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok());
    Picked {
        kind: ClipKind::Image,
        text: None,
        hash: content_hash(ClipKind::Image, data),
        metadata: size.map(|(w, h)| json!({ "w": w, "h": h })),
    }
}

/// The text shows file names, one per line. The hash covers the sorted full paths, so the
/// same files copied in another order are the same clip.
fn files(paths: Vec<String>) -> Picked {
    let names: Vec<&str> = paths.iter().map(|p| file_name(p)).collect();
    let text = names.join("\n");
    let metadata = json!({ "count": paths.len(), "names": names });
    let mut sorted = paths.clone();
    sorted.sort();
    Picked {
        kind: ClipKind::File,
        text: Some(text),
        hash: content_hash(ClipKind::File, sorted.join("\n").as_bytes()),
        metadata: Some(metadata),
    }
}

/// Pasteboard text is UTF-8. Some apps end it with a NUL.
fn decode(data: &[u8]) -> String {
    String::from_utf8_lossy(data)
        .trim_end_matches('\0')
        .to_owned()
}

/// `file:///Users/me/My%20Notes.txt` as `/Users/me/My Notes.txt`.
pub(crate) fn file_path(data: &[u8]) -> Option<String> {
    let url = decode(data);
    let rest = url.trim().strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let path = percent_decode(rest)?;
    Some(path).filter(|p| !p.contains('\0'))
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The last component, keeping a folder's name: `/a/Folder/` gives `Folder`.
fn file_name(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rsplit('/').next() {
        Some(name) if !name.is_empty() => name,
        _ => "/",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(item_index: u32, format: &str, data: &[u8]) -> RawFormat {
        RawFormat {
            item_index,
            format: format.to_owned(),
            data: data.to_vec(),
        }
    }

    fn text(text: &str) -> Vec<RawFormat> {
        vec![raw(0, uti::PLAIN_TEXT, text.as_bytes())]
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        image::RgbaImage::new(w, h)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    fn kind_of(formats: Vec<RawFormat>) -> Option<ClipKind> {
        classify(formats).map(|c| c.kind)
    }

    #[test]
    fn kinds() {
        use ClipKind::*;
        let html = |t: &str| {
            vec![
                raw(0, uti::PLAIN_TEXT, t.as_bytes()),
                raw(0, uti::HTML, format!("<b>{t}</b>").as_bytes()),
            ]
        };
        let cases: Vec<(&str, Vec<RawFormat>, Option<ClipKind>)> = vec![
            ("plain text", text("Hello world"), Some(Text)),
            ("whitespace is still text", text(" \n"), Some(Text)),
            ("text with html", html("Hello world"), Some(RichText)),
            (
                "text with rtf",
                vec![
                    raw(0, uti::PLAIN_TEXT, b"Hello"),
                    raw(0, uti::RTF, b"{\\rtf1 Hello}"),
                ],
                Some(RichText),
            ),
            ("link", text("https://example.com/a"), Some(Link)),
            (
                "link with spaces round it",
                text("  https://example.com\n"),
                Some(Link),
            ),
            (
                "link from a web page keeps link",
                html("https://example.com"),
                Some(Link),
            ),
            (
                "link inside prose",
                text("see https://example.com"),
                Some(Text),
            ),
            ("hex color", text("#1E90FF"), Some(Color)),
            ("css color", text("rgb(30, 144, 255)"), Some(Color)),
            ("code", text("fn main() {\n    run();\n}"), Some(Code)),
            (
                "code from an editor with html",
                html("let x = foo(1);"),
                Some(Code),
            ),
            (
                "url format wins over the text",
                vec![
                    raw(0, uti::PLAIN_TEXT, b"Example Domain"),
                    raw(0, uti::URL, b"https://example.com/"),
                ],
                Some(Link),
            ),
            (
                "url format that isn't a web link",
                vec![
                    raw(0, uti::PLAIN_TEXT, b"me@example.com"),
                    raw(0, uti::URL, b"mailto:me@example.com"),
                ],
                Some(Text),
            ),
            (
                "url format alone",
                vec![raw(0, uti::URL, b"https://example.com")],
                Some(Link),
            ),
            ("png", vec![raw(0, uti::PNG, &png(2, 3))], Some(Image)),
            (
                "text wins over an image",
                vec![
                    raw(0, uti::PLAIN_TEXT, b"A1\tB1"),
                    raw(0, uti::PNG, &png(1, 1)),
                ],
                Some(Text),
            ),
            (
                "image with html and no text",
                vec![
                    raw(0, uti::HTML, b"<img src=\"a.png\">"),
                    raw(0, uti::PNG, &png(1, 1)),
                ],
                Some(Image),
            ),
            (
                "file",
                vec![
                    raw(0, uti::FILE_URL, b"file:///Users/me/a.txt"),
                    raw(0, uti::PLAIN_TEXT, b"a.txt"),
                    raw(0, uti::TIFF, b"icon"),
                ],
                Some(File),
            ),
            (
                "rich text only",
                vec![raw(0, uti::RTF, b"{\\rtf1 Hi}")],
                Some(RichText),
            ),
            ("empty text", text(""), None),
            ("nothing", vec![], None),
            (
                "unknown format",
                vec![raw(0, "com.example.private", b"x")],
                None,
            ),
            (
                "file url that isn't a path",
                vec![raw(0, uti::FILE_URL, b"https://x.com")],
                None,
            ),
        ];
        for (name, formats, want) in cases {
            assert_eq!(kind_of(formats), want, "{name}");
        }
    }

    #[test]
    fn plain_text_and_metadata() {
        let link = classify(text(" https://Example.com/a?b=1 \n")).unwrap();
        assert_eq!(
            link.plain_text.as_deref(),
            Some("https://Example.com/a?b=1")
        );
        assert_eq!(link.metadata, Some(json!({ "host": "example.com" })));

        let color = classify(text("rgba(30, 144, 255, 0.5)")).unwrap();
        assert_eq!(color.plain_text.as_deref(), Some("#1E90FF80"));
        assert_eq!(color.metadata, Some(json!({ "rgba": [30, 144, 255, 128] })));

        let rust = classify(text("fn main() {\n    let mut x = 1;\n}")).unwrap();
        assert_eq!(rust.metadata, Some(json!({ "lang": "rust" })));
        let unknown = classify(text("x = foo(bar);")).unwrap();
        assert_eq!((unknown.kind, unknown.metadata), (ClipKind::Code, None));

        let image = classify(vec![raw(0, uti::PNG, &png(4, 3))]).unwrap();
        assert_eq!(image.plain_text, None);
        assert_eq!(image.metadata, Some(json!({ "w": 4, "h": 3 })));
        let broken = classify(vec![raw(0, uti::PNG, b"not a png")]).unwrap();
        assert_eq!((broken.kind, broken.metadata), (ClipKind::Image, None));

        let files = classify(vec![
            raw(0, uti::FILE_URL, b"file:///Users/me/My%20Notes.txt"),
            raw(1, uti::FILE_URL, b"file://localhost/Users/me/Photos/"),
            raw(2, uti::FILE_URL, b"file:///Users/me/caf%C3%A9.md"),
        ])
        .unwrap();
        assert_eq!(
            files.plain_text.as_deref(),
            Some("My Notes.txt\nPhotos\ncafé.md")
        );
        assert_eq!(
            files.metadata,
            Some(json!({ "count": 3, "names": ["My Notes.txt", "Photos", "café.md"] }))
        );

        let text = classify(text("Hello\0")).unwrap();
        assert_eq!(text.plain_text.as_deref(), Some("Hello"));
    }

    #[test]
    fn color_archive_makes_a_color() {
        // NSColor(deviceRed: 1, green: 0, blue: 0, alpha: 1), from color.rs's tests.
        let archive = color::tests::unhex(color::tests::DEVICE);
        let clip = classify(vec![raw(0, uti::COLOR, &archive)]).unwrap();
        assert_eq!(clip.kind, ClipKind::Color);
        assert_eq!(clip.plain_text.as_deref(), Some("#FF0000FF"));
        assert_eq!(
            clip.content_hash,
            classify(text("#f00")).unwrap().content_hash
        );
        // An archive that doesn't decode falls through to the text with it.
        let clip = classify(vec![
            raw(0, uti::COLOR, b"garbage"),
            raw(0, uti::PLAIN_TEXT, b"red"),
        ])
        .unwrap();
        assert_eq!(clip.kind, ClipKind::Text);
    }

    #[test]
    fn plain_text_is_cut_but_the_hash_covers_everything() {
        use crate::storage::MAX_TEXT_INDEX_BYTES;
        let long = "é".repeat(MAX_TEXT_INDEX_BYTES);
        let mut longer = long.clone();
        longer.push('!');
        let a = classify(text(&long)).unwrap();
        let b = classify(text(&longer)).unwrap();
        let cut = a.plain_text.as_deref().unwrap();
        assert!(cut.len() <= MAX_TEXT_INDEX_BYTES && cut.len() > MAX_TEXT_INDEX_BYTES - 4);
        assert_eq!(a.plain_text, b.plain_text);
        assert_ne!(a.content_hash, b.content_hash);
        assert_eq!(a.formats[0].data.len(), long.len());
    }

    #[test]
    fn hashes_follow_the_data_model() {
        let clip = classify(text("Hello")).unwrap();
        assert_eq!(clip.content_hash, content_hash(ClipKind::Text, b"Hello"));
        let manual = {
            let mut h = blake3::Hasher::new();
            h.update(b"text\0Hello");
            h.finalize().to_hex().to_string()
        };
        assert_eq!(clip.content_hash, manual);

        let image = png(2, 2);
        let clip = classify(vec![raw(0, uti::PNG, &image), raw(0, uti::TIFF, b"tiff")]).unwrap();
        assert_eq!(clip.content_hash, content_hash(ClipKind::Image, &image));
        let clip = classify(vec![raw(0, uti::TIFF, b"tiff")]).unwrap();
        assert_eq!(clip.content_hash, content_hash(ClipKind::Image, b"tiff"));

        let clip = classify(vec![
            raw(0, uti::FILE_URL, b"file:///b"),
            raw(1, uti::FILE_URL, b"file:///a"),
        ])
        .unwrap();
        assert_eq!(clip.content_hash, content_hash(ClipKind::File, b"/a\n/b"));
    }

    #[test]
    fn duplicates_share_a_hash() {
        let hash = |formats| classify(formats).unwrap().content_hash;
        // Same text, same hash. Whitespace counts, because it matters in code.
        assert_eq!(hash(text("Hello")), hash(text("Hello")));
        assert_ne!(hash(text("Hello")), hash(text("Hello ")));
        // The kind is part of the hash: plain and rich copies of the same words are two clips.
        assert_ne!(
            hash(text("Hello")),
            hash(vec![
                raw(0, uti::PLAIN_TEXT, b"Hello"),
                raw(0, uti::HTML, b"<i>Hello</i>")
            ])
        );
        // Rich text is one clip whatever its formatting; the store keeps the latest.
        assert_eq!(
            hash(vec![
                raw(0, uti::PLAIN_TEXT, b"Hello"),
                raw(0, uti::HTML, b"<i>Hello</i>")
            ]),
            hash(vec![
                raw(0, uti::PLAIN_TEXT, b"Hello"),
                raw(0, uti::RTF, b"{\\rtf1 Hello}")
            ])
        );
        // Colors are normalised, so every spelling of one color is one clip.
        assert_eq!(hash(text("#1e90ff")), hash(text("rgb(30 144 255)")));
        assert_eq!(hash(text("#1E90FF")), hash(text("#1E90FFFF")));
        // Links are trimmed, but otherwise compared as written.
        assert_eq!(
            hash(text("https://example.com")),
            hash(text("https://example.com\n"))
        );
        assert_ne!(
            hash(text("https://example.com")),
            hash(text("https://example.com/"))
        );
        // Files in any order.
        assert_eq!(
            hash(vec![
                raw(0, uti::FILE_URL, b"file:///a"),
                raw(1, uti::FILE_URL, b"file:///b")
            ]),
            hash(vec![
                raw(0, uti::FILE_URL, b"file:///b"),
                raw(1, uti::FILE_URL, b"file:///a")
            ])
        );
    }

    #[test]
    fn every_format_is_kept_and_counted() {
        let formats = vec![
            raw(0, uti::PLAIN_TEXT, b"Hello"),
            raw(0, uti::HTML, b"<b>Hello</b>"),
        ];
        let clip = classify(formats.clone()).unwrap();
        assert_eq!(clip.formats.len(), 2);
        assert_eq!(clip.size_bytes(), 17);
        let source = AppRef {
            bundle_id: Some("com.apple.Safari".into()),
            name: Some("Safari".into()),
        };
        let new = clip.into_clip(Some(source));
        assert_eq!(new.source_app_name.as_deref(), Some("Safari"));
        assert_eq!(new.source_app_id.as_deref(), Some("com.apple.Safari"));
        assert_eq!(new.platform, Platform::MacOs);
    }

    #[test]
    fn kinds_stay_within_what_the_formats_allow() {
        use crate::privacy::possible_kinds;
        use kopi_platform::PasteboardTypes;
        let samples: Vec<Vec<RawFormat>> = vec![
            text("Hello"),
            text("https://example.com"),
            text("#fff"),
            text("let x = foo(1);"),
            vec![raw(0, uti::URL, b"https://example.com")],
            vec![raw(0, uti::PNG, &png(1, 1))],
            vec![raw(0, uti::TIFF, b"x"), raw(0, uti::HTML, b"<img>")],
            vec![raw(0, uti::FILE_URL, b"file:///a")],
            vec![raw(0, uti::RTF, b"{\\rtf1 x}")],
        ];
        for formats in samples {
            let types = PasteboardTypes {
                change_count: 1,
                items: vec![formats.iter().map(|f| f.format.clone()).collect()],
            };
            let kind = kind_of(formats).unwrap();
            assert!(possible_kinds(&types).contains(kind), "{kind}");
        }
    }
}
