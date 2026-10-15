//! The Win32 clipboard behind the [`Pasteboard`] trait, in macOS format names.
//!
//! | Windows | Kopi |
//! |---|---|
//! | `CF_UNICODETEXT` | `public.utf8-plain-text`, as UTF-8 |
//! | `HTML Format` | `public.html`, the fragment without the CF_HTML header |
//! | `Rich Text Format` | `public.rtf` |
//! | `PNG`, else `CF_DIB` | `public.png`. A bitmap is converted. |
//! | `UniformResourceLocatorW` | `public.url` |
//! | `CF_HDROP` | `public.file-url`, one item per file, as on macOS |
//! | `ExcludeClipboardContentFromMonitorProcessing`, or `CanIncludeInClipboardHistory` set to 0 | `org.nspasteboard.ConcealedType` |
//! | `Clipboard Viewer Ignore` | `org.nspasteboard.TransientType` |
//!
//! Other registered formats keep their names, so Kopi's own source marker is
//! `org.nspasteboard.source` on Windows too. Writing turns each name back into its format.

use std::io::Cursor;
use std::thread;
use std::time::Duration;

use image::codecs::bmp::BmpDecoder;
use image::{DynamicImage, ImageFormat};
use windows_sys::Win32::Foundation::{GlobalFree, POINT};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardFormatNameW, GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::UI::Shell::{DROPFILES, DragQueryFileW};

use super::{from_wide, wide};
use crate::pasteboard::{
    Pasteboard, PasteboardAccess, PasteboardError, PasteboardTypes, RawFormat, uti,
};

const CF_DIB: u32 = 8;
const CF_HDROP: u32 = 15;
const CF_UNICODETEXT: u32 = 13;
const CF_DIBV5: u32 = 17;
/// Format IDs from here up are registered by name.
const FIRST_REGISTERED: u32 = 0xC000;

const HTML: &str = "HTML Format";
const RTF: &str = "Rich Text Format";
const PNG: &str = "PNG";
const URL: &str = "UniformResourceLocatorW";
/// How Explorer is told a file drop is a copy, not a move.
const DROP_EFFECT: &str = "Preferred DropEffect";
const DROPEFFECT_COPY: u32 = 1;

/// Windows' privacy signals, and what they're reported as.
const EXCLUDE: &str = "ExcludeClipboardContentFromMonitorProcessing";
const HISTORY: &str = "CanIncludeInClipboardHistory";
const VIEWER_IGNORE: &str = "Clipboard Viewer Ignore";
const CONCEALED: &str = "org.nspasteboard.ConcealedType";
const TRANSIENT: &str = "org.nspasteboard.TransientType";

/// How often, and how far apart, to try opening the clipboard while another app has it open.
const OPEN_ATTEMPTS: u32 = 20;
const OPEN_RETRY: Duration = Duration::from_millis(10);

/// The Windows clipboard. Calls can come from any thread: each opens the clipboard, works and
/// closes it again.
#[derive(Debug, Clone, Default)]
pub struct WinPasteboard;

impl WinPasteboard {
    /// The clipboard. Windows has only the one.
    pub fn general() -> Self {
        Self
    }
}

impl Pasteboard for WinPasteboard {
    fn change_count(&self) -> Result<u64, PasteboardError> {
        Ok(sequence())
    }

    fn types(&self) -> Result<PasteboardTypes, PasteboardError> {
        let _open = Open::new()?;
        let change_count = sequence();
        let mut names: Vec<String> = Vec::new();
        let mut files = 0;
        let (mut png, mut bitmap) = (false, false);
        let mut format = 0;
        loop {
            // SAFETY: the clipboard is open.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            let name = match format {
                CF_UNICODETEXT => uti::PLAIN_TEXT.to_owned(),
                CF_DIB | CF_DIBV5 => {
                    bitmap = true;
                    continue;
                }
                CF_HDROP => {
                    files = file_paths().len();
                    continue;
                }
                f if f >= FIRST_REGISTERED => match format_name(f) {
                    Some(name) => match name.as_str() {
                        HTML => uti::HTML.to_owned(),
                        RTF => uti::RTF.to_owned(),
                        PNG => {
                            png = true;
                            uti::PNG.to_owned()
                        }
                        URL => uti::URL.to_owned(),
                        EXCLUDE => CONCEALED.to_owned(),
                        VIEWER_IGNORE => TRANSIENT.to_owned(),
                        HISTORY if dword(f) == Some(0) => CONCEALED.to_owned(),
                        _ => name,
                    },
                    None => continue,
                },
                // Formats Kopi neither reads nor checks, and the ones Windows makes from
                // another, such as CF_TEXT from CF_UNICODETEXT.
                _ => continue,
            };
            if !names.contains(&name) {
                names.push(name);
            }
        }
        if bitmap && !png {
            names.push(uti::PNG.to_owned());
        }
        let mut items = vec![names];
        if files > 0 {
            items[0].push(uti::FILE_URL.to_owned());
            items.extend((1..files).map(|_| vec![uti::FILE_URL.to_owned()]));
        }
        if items[0].is_empty() && items.len() == 1 {
            items.clear();
        }
        Ok(PasteboardTypes {
            change_count,
            items,
        })
    }

    fn read(
        &self,
        expected: u64,
        formats: &[Vec<String>],
        max_bytes: u64,
    ) -> Result<Vec<RawFormat>, PasteboardError> {
        let _open = Open::new()?;
        if sequence() != expected {
            return Err(PasteboardError::Changed);
        }
        let mut budget = Budget(max_bytes);
        let mut out = Vec::new();
        let mut paths: Option<Vec<String>> = None;
        for (index, wanted) in formats.iter().enumerate() {
            for format in wanted {
                let data = if format == uti::FILE_URL {
                    let paths = paths.get_or_insert_with(file_paths);
                    paths.get(index).map(|path| file_url(path).into_bytes())
                } else if index > 0 {
                    // Only files have more than one item.
                    None
                } else {
                    read_format(format, &budget)?
                };
                let Some(data) = data else {
                    continue;
                };
                budget.take(data.len())?;
                out.push(RawFormat {
                    item_index: index as u32,
                    format: format.clone(),
                    data,
                });
            }
        }
        if sequence() != expected {
            return Err(PasteboardError::Changed);
        }
        Ok(out)
    }

    /// Windows has no permission for reading the clipboard.
    fn access(&self) -> Result<PasteboardAccess, PasteboardError> {
        Ok(PasteboardAccess::AlwaysAllow)
    }

    fn write(&self, formats: &[RawFormat], source: Option<&str>) -> Result<u64, PasteboardError> {
        let mut entries: Vec<(u32, Vec<u8>)> = Vec::new();
        let mut paths = Vec::new();
        for f in formats {
            match f.format.as_str() {
                uti::PLAIN_TEXT => entries.push((CF_UNICODETEXT, utf16(&f.data))),
                uti::HTML => entries.push((registered(HTML), cf_html(&f.data))),
                uti::RTF => entries.push((registered(RTF), f.data.clone())),
                uti::PNG => {
                    entries.push((registered(PNG), f.data.clone()));
                    // Paint and most older apps take only a bitmap.
                    if let Some(dib) = png_to_dib(&f.data) {
                        entries.push((CF_DIB, dib));
                    }
                }
                uti::URL => entries.push((registered(URL), utf16(&f.data))),
                uti::FILE_URL => {
                    if let Some(path) = windows_path(&f.data) {
                        paths.push(path);
                    }
                }
                // macOS-only formats, such as RTFD and colors, have no Windows twin.
                uti::RTFD | uti::TIFF | uti::COLOR => {}
                other => entries.push((registered(other), f.data.clone())),
            }
        }
        if !paths.is_empty() {
            entries.push((CF_HDROP, drop_files(&paths)));
            entries.push((
                registered(DROP_EFFECT),
                DROPEFFECT_COPY.to_le_bytes().to_vec(),
            ));
        }
        if let Some(source) = source {
            entries.push((registered(uti::SOURCE), source.as_bytes().to_vec()));
        }

        let open = Open::new()?;
        // SAFETY: the clipboard is open.
        if unsafe { EmptyClipboard() } == 0 {
            return Err(PasteboardError::WriteFailed);
        }
        for (format, data) in entries {
            set_data(format, &data)?;
        }
        drop(open);
        Ok(sequence())
    }
}

/// The clipboard, open until dropped. Only one app at a time may have it open, so opening
/// waits a little for whoever has it.
struct Open;

impl Open {
    fn new() -> Result<Self, PasteboardError> {
        for attempt in 0..OPEN_ATTEMPTS {
            if attempt > 0 {
                thread::sleep(OPEN_RETRY);
            }
            // SAFETY: no owner window. Kopi never delays rendering, so it needs none.
            if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                return Ok(Self);
            }
        }
        Err(PasteboardError::Busy)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: opened by this thread in `new`.
        unsafe { CloseClipboard() };
    }
}

/// What's left of the size limit for a read.
struct Budget(u64);

impl Budget {
    fn check(&self, bytes: usize) -> Result<(), PasteboardError> {
        if bytes as u64 > self.0 {
            return Err(PasteboardError::TooLarge);
        }
        Ok(())
    }

    fn take(&mut self, bytes: usize) -> Result<(), PasteboardError> {
        self.check(bytes)?;
        self.0 -= bytes as u64;
        Ok(())
    }
}

fn sequence() -> u64 {
    // SAFETY: takes nothing. Needs no open clipboard.
    u64::from(unsafe { GetClipboardSequenceNumber() })
}

/// The name a format was registered with.
fn format_name(format: u32) -> Option<String> {
    let mut buf = [0u16; 256];
    // SAFETY: `buf` holds `buf.len()` characters.
    let len = unsafe { GetClipboardFormatNameW(format, buf.as_mut_ptr(), buf.len() as i32) };
    (len > 0).then(|| from_wide(&buf[..len as usize]))
}

/// The ID for a format name, registering it if no app has yet.
fn registered(name: &str) -> u32 {
    // SAFETY: a NUL-terminated string.
    unsafe { RegisterClipboardFormatW(wide(name).as_ptr()) }
}

/// Reads one Kopi format from the open clipboard. The read stops at the limit before copying.
fn read_format(format: &str, budget: &Budget) -> Result<Option<Vec<u8>>, PasteboardError> {
    let raw = |id: u32, budget: &Budget| -> Result<Option<Vec<u8>>, PasteboardError> {
        if id == 0 {
            return Ok(None);
        }
        data(id, budget)
    };
    Ok(match format {
        uti::PLAIN_TEXT => raw(CF_UNICODETEXT, budget)?.map(|d| from_utf16(&d).into_bytes()),
        uti::HTML => raw(registered(HTML), budget)?.and_then(|d| html_fragment(&d)),
        uti::RTF => raw(registered(RTF), budget)?.map(trim_nul),
        uti::URL => raw(registered(URL), budget)?.map(|d| from_utf16(&d).into_bytes()),
        uti::PNG => match raw(registered(PNG), budget)? {
            Some(png) => Some(png),
            None => raw(CF_DIB, budget)?.and_then(|dib| dib_to_png(&dib)),
        },
        other => raw(registered(other), budget)?,
    })
}

/// Copies a format's data out of the open clipboard, or `None` if it isn't there.
fn data(format: u32, budget: &Budget) -> Result<Option<Vec<u8>>, PasteboardError> {
    // SAFETY: the clipboard is open, and the handle is only used while it is. The data is
    // copied out between lock and unlock.
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return Ok(None);
        }
        let size = GlobalSize(handle);
        budget.check(size)?;
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            return Ok(None);
        }
        let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), size).to_vec();
        GlobalUnlock(handle);
        Ok(Some(bytes))
    }
}

/// A DWORD format, such as `CanIncludeInClipboardHistory`.
fn dword(format: u32) -> Option<u32> {
    let bytes = data(format, &Budget(64)).ok().flatten()?;
    Some(u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?))
}

/// Puts `bytes` on the open, emptied clipboard. The clipboard owns the memory once it takes it.
fn set_data(format: u32, bytes: &[u8]) -> Result<(), PasteboardError> {
    if format == 0 {
        return Err(PasteboardError::WriteFailed);
    }
    // SAFETY: the memory is ours until SetClipboardData takes it, and is freed if it doesn't.
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
        if handle.is_null() {
            return Err(PasteboardError::WriteFailed);
        }
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            GlobalFree(handle);
            return Err(PasteboardError::WriteFailed);
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
        GlobalUnlock(handle);
        if SetClipboardData(format, handle).is_null() {
            GlobalFree(handle);
            return Err(PasteboardError::WriteFailed);
        }
    }
    Ok(())
}

/// The paths in `CF_HDROP`, if there is one.
fn file_paths() -> Vec<String> {
    // SAFETY: the clipboard is open. DragQueryFileW reads the handle without taking it.
    unsafe {
        let handle = GetClipboardData(CF_HDROP);
        if handle.is_null() {
            return Vec::new();
        }
        let count = DragQueryFileW(handle, u32::MAX, std::ptr::null_mut(), 0);
        (0..count)
            .filter_map(|i| {
                let len = DragQueryFileW(handle, i, std::ptr::null_mut(), 0);
                if len == 0 {
                    return None;
                }
                let mut buf = vec![0u16; len as usize + 1];
                let got = DragQueryFileW(handle, i, buf.as_mut_ptr(), buf.len() as u32);
                (got > 0).then(|| from_wide(&buf))
            })
            .collect()
    }
}

/// A `DROPFILES` followed by the paths, each NUL-terminated, and a NUL to end the list.
fn drop_files(paths: &[String]) -> Vec<u8> {
    let header = DROPFILES {
        pFiles: size_of::<DROPFILES>() as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: 0,
        fWide: 1,
    };
    // SAFETY: DROPFILES is plain data.
    let mut out = unsafe {
        std::slice::from_raw_parts((&raw const header).cast::<u8>(), size_of::<DROPFILES>())
    }
    .to_vec();
    for path in paths {
        for unit in path.encode_utf16().chain(std::iter::once(0)) {
            out.extend_from_slice(&unit.to_le_bytes());
        }
    }
    out.extend_from_slice(&[0, 0]);
    out
}

/// `C:\Users\me\a b.txt` as `file:///C:/Users/me/a%20b.txt`, and `\\server\share\a` as
/// `file://server/share/a`.
fn file_url(path: &str) -> String {
    let slashed = path.replace('\\', "/");
    let (prefix, rest) = match slashed.strip_prefix("//") {
        Some(share) => ("file://", share.to_owned()),
        None => ("file:///", slashed),
    };
    let mut url = String::from(prefix);
    for byte in rest.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'-' | b'_' | b'.' | b'~' => {
                url.push(byte as char)
            }
            _ => url.push_str(&format!("%{byte:02X}")),
        }
    }
    url
}

/// The Windows path in a file URL, for `CF_HDROP`. The inverse of [`file_url`].
fn windows_path(url: &[u8]) -> Option<String> {
    let url = std::str::from_utf8(url).ok()?.trim().trim_end_matches('\0');
    let rest = url.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let decoded = percent_decode(rest)?;
    let path = match decoded.strip_prefix('/') {
        // A drive: /C:/a
        Some(local) if local.as_bytes().get(1) == Some(&b':') => local.replace('/', "\\"),
        // A path from macOS has nothing to point to here.
        Some(_) => return None,
        None => format!("\\\\{}", decoded.replace('/', "\\")),
    };
    Some(path)
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            out.push(u8::from_str_radix(text.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// UTF-8 as NUL-terminated UTF-16LE, for `CF_UNICODETEXT`.
fn utf16(text: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(text)
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// UTF-16LE up to the first NUL.
fn from_utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    from_wide(&units)
}

fn trim_nul(mut data: Vec<u8>) -> Vec<u8> {
    while data.last() == Some(&0) {
        data.pop();
    }
    data
}

/// The fragment in CF_HTML: the bytes between `StartFragment` and `EndFragment`, offsets the
/// header gives.
fn html_fragment(data: &[u8]) -> Option<Vec<u8>> {
    let header = String::from_utf8_lossy(&data[..data.len().min(512)]);
    let offset = |key: &str| -> Option<usize> {
        let line = header.lines().find_map(|l| l.strip_prefix(key))?;
        line.trim().parse().ok()
    };
    let start = offset("StartFragment:")?;
    let end = offset("EndFragment:")?;
    (start <= end && end <= data.len()).then(|| data[start..end].to_vec())
}

/// HTML wrapped in CF_HTML: a header of byte offsets, then a document around the fragment.
fn cf_html(fragment: &[u8]) -> Vec<u8> {
    const PREFIX: &str = "<html><body>\r\n<!--StartFragment-->";
    const SUFFIX: &str = "<!--EndFragment-->\r\n</body></html>";
    let header = |a: usize, b: usize, c: usize, d: usize| {
        format!(
            "Version:0.9\r\nStartHTML:{a:010}\r\nEndHTML:{b:010}\r\nStartFragment:{c:010}\r\nEndFragment:{d:010}\r\n"
        )
    };
    let header_len = header(0, 0, 0, 0).len();
    let start_fragment = header_len + PREFIX.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + SUFFIX.len();
    let mut out = header(header_len, end_html, start_fragment, end_fragment).into_bytes();
    out.extend_from_slice(PREFIX.as_bytes());
    out.extend_from_slice(fragment);
    out.extend_from_slice(SUFFIX.as_bytes());
    out
}

/// A `CF_DIB` (a BMP without its file header) as PNG.
fn dib_to_png(dib: &[u8]) -> Option<Vec<u8>> {
    let decoder = BmpDecoder::new_without_file_header(Cursor::new(dib)).ok()?;
    let image = DynamicImage::from_decoder(decoder).ok()?;
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .ok()?;
    Some(png)
}

/// A PNG as a 24-bit `CF_DIB`. The PNG goes on the clipboard too, for apps that keep alpha.
fn png_to_dib(png: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory_with_format(png, ImageFormat::Png).ok()?;
    let mut bmp = Vec::new();
    DynamicImage::ImageRgb8(image.to_rgb8())
        .write_to(&mut Cursor::new(&mut bmp), ImageFormat::Bmp)
        .ok()?;
    // Drops the 14-byte BITMAPFILEHEADER.
    bmp.get(14..).map(<[u8]>::to_vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_urls_round_trip() {
        for (path, url) in [
            (r"C:\Users\me\a b.txt", "file:///C:/Users/me/a%20b.txt"),
            (r"C:\café", "file:///C:/caf%C3%A9"),
            (r"\\server\share\x", "file://server/share/x"),
        ] {
            assert_eq!(file_url(path), url);
            assert_eq!(windows_path(url.as_bytes()).as_deref(), Some(path));
        }
        assert_eq!(windows_path(b"file:///Users/me/a.txt"), None);
    }

    #[test]
    fn cf_html_round_trips() {
        let fragment = "<b>Bold</b> café".as_bytes();
        let html = cf_html(fragment);
        assert_eq!(html_fragment(&html).as_deref(), Some(fragment));
        let text = String::from_utf8(html).unwrap();
        assert!(text.starts_with("Version:0.9\r\nStartHTML:0000000105\r\n"));
        assert!(text.ends_with("</body></html>"));
    }

    #[test]
    fn text_is_utf16() {
        let bytes = utf16("hé".as_bytes());
        assert_eq!(bytes, [b'h', 0, 0xE9, 0, 0, 0]);
        assert_eq!(from_utf16(&bytes), "hé");
    }

    #[test]
    fn images_convert() {
        let mut png = Vec::new();
        DynamicImage::new_rgb8(3, 2)
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        let dib = png_to_dib(&png).unwrap();
        let back = image::load_from_memory(&dib_to_png(&dib).unwrap()).unwrap();
        assert_eq!((back.width(), back.height()), (3, 2));
    }
}
