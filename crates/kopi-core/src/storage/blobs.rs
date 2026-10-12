//! Content-addressed files for format data over 64 KiB.
//!
//! A blob lives at `blobs/3f/a9/3fa9…e1.bin`, named by the BLAKE3 hex of its bytes. Two clips
//! with the same image share one file.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{StorageError, StorageResult};

#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// The file for `hash`. Fails if `hash` isn't 64 lowercase hex characters, so a hash from
    /// a URL can't point outside the folder.
    pub fn path(&self, hash: &str) -> StorageResult<PathBuf> {
        check_hash(hash)?;
        Ok(self
            .root
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(format!("{hash}.bin")))
    }

    /// Writes `data` and returns its hash. Does nothing if the file is already there.
    ///
    /// The file is synced and then renamed into place, so a crash never leaves a partial blob
    /// under its final name.
    pub fn put(&self, data: &[u8]) -> StorageResult<String> {
        let hash = blake3::hash(data).to_hex().to_string();
        let path = self.path(&hash)?;
        if path.exists() {
            return Ok(hash);
        }
        write_atomic(&path, data)?;
        Ok(hash)
    }

    pub fn read(&self, hash: &str) -> StorageResult<Vec<u8>> {
        Ok(fs::read(self.path(hash)?)?)
    }

    /// Deletes the file for `hash`. A missing file isn't an error.
    pub fn remove(&self, hash: &str) -> StorageResult<()> {
        match fs::remove_file(self.path(hash)?) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }
}

pub(crate) fn check_hash(hash: &str) -> StorageResult<()> {
    let ok = hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if ok {
        Ok(())
    } else {
        Err(StorageError::InvalidHash(hash.to_owned()))
    }
}

/// Writes to a temporary file next to `path`, syncs it and renames it into place.
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let dir = path
        .parent()
        .expect("blob and thumbnail paths have a parent");
    fs::create_dir_all(dir)?;
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
