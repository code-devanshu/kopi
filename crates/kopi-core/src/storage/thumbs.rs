//! PNG thumbnails for image clips, made on a background thread.
//!
//! A thumbnail lives at `thumbs/3f/3fa9…e1@320.png`, named by the clip's content hash. The list
//! only ever loads thumbnails. The full image is read only for the preview or a paste.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use image::codecs::bmp::BmpDecoder;
use image::{DynamicImage, ImageFormat, ImageReader, Limits};

use super::blobs::write_atomic;

/// The longest edge of a thumbnail, in pixels.
pub const THUMBNAIL_SIZE: u32 = 320;

/// Sent to the listener when a thumbnail is written or fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailDone {
    pub content_hash: String,
    /// The thumbnail file, or why it couldn't be made.
    pub result: Result<PathBuf, String>,
}

pub type ThumbnailListener = Box<dyn Fn(ThumbnailDone) + Send + 'static>;

/// The encodings we can make a thumbnail from, in the order we prefer them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceFormat {
    Png,
    Tiff,
    /// A Windows `CF_DIBV5`: a bitmap without the BMP file header.
    Dib,
}

impl SourceFormat {
    pub(crate) fn from_format_name(name: &str) -> Option<Self> {
        match name {
            "public.png" | "PNG" => Some(Self::Png),
            "public.tiff" => Some(Self::Tiff),
            "CF_DIBV5" => Some(Self::Dib),
            _ => None,
        }
    }
}

pub(crate) enum Source {
    Bytes(Vec<u8>),
    File(PathBuf),
}

pub(crate) struct Job {
    pub content_hash: String,
    pub format: SourceFormat,
    pub source: Source,
}

/// Owns the worker thread. Dropping it finishes the queued jobs and joins the thread.
pub(crate) struct Thumbnailer {
    tx: Option<Sender<Job>>,
    worker: Option<JoinHandle<()>>,
}

impl Thumbnailer {
    pub(crate) fn spawn(dir: PathBuf, listener: ThumbnailListener) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel::<Job>();
        let worker = thread::Builder::new()
            .name("kopi-thumbnails".into())
            .spawn(move || {
                for job in rx {
                    let path = thumbnail_path(&dir, &job.content_hash);
                    let result = make(&job)
                        .and_then(|png| write_atomic(&path, &png).map_err(|e| e.to_string()))
                        .map(|()| path);
                    listener(ThumbnailDone {
                        content_hash: job.content_hash,
                        result,
                    });
                }
            })?;
        Ok(Self {
            tx: Some(tx),
            worker: Some(worker),
        })
    }

    pub(crate) fn queue(&self, job: Job) {
        if let Some(tx) = &self.tx {
            // The worker only stops when we drop the sender, so this can't fail.
            let _ = tx.send(job);
        }
    }
}

impl Drop for Thumbnailer {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Where the thumbnail for a clip lives. `content_hash` must already be checked.
pub(crate) fn thumbnail_path(dir: &Path, content_hash: &str) -> PathBuf {
    dir.join(&content_hash[..2])
        .join(format!("{content_hash}@{THUMBNAIL_SIZE}.png"))
}

fn make(job: &Job) -> Result<Vec<u8>, String> {
    let bytes = match &job.source {
        Source::Bytes(bytes) => std::borrow::Cow::Borrowed(bytes),
        Source::File(path) => std::fs::read(path)
            .map(std::borrow::Cow::Owned)
            .map_err(|e| e.to_string())?,
    };
    let image = decode(&bytes, job.format).map_err(|e| e.to_string())?;
    let image = if image.width() > THUMBNAIL_SIZE || image.height() > THUMBNAIL_SIZE {
        image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE)
    } else {
        image
    };
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(image.to_rgba8())
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

fn decode(bytes: &[u8], format: SourceFormat) -> image::ImageResult<DynamicImage> {
    let format = match format {
        SourceFormat::Png => ImageFormat::Png,
        SourceFormat::Tiff => ImageFormat::Tiff,
        SourceFormat::Dib => {
            let mut decoder = BmpDecoder::new_without_file_header(Cursor::new(bytes))?;
            image::ImageDecoder::set_limits(&mut decoder, Limits::default())?;
            return DynamicImage::from_decoder(decoder);
        }
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    // The defaults cap decoding at 512 MiB, which stops a tiny file that claims huge dimensions.
    reader.limits(Limits::default());
    reader.decode()
}
