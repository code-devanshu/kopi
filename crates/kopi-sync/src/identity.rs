//! This computer's sync identity: a device ID and a static X25519 key pair, in
//! `<data dir>/sync/device.key`. Made the first time sync is turned on.
//!
//! Not the Keychain or Credential Manager: the key protects nothing that `kopi.db`, beside it,
//! doesn't already expose, and keychain items on a self-signed app risk an access prompt after
//! updates. See `docs/SYNC_DESIGN.md`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::noise;

#[derive(Clone)]
pub struct Identity {
    pub device_id: String,
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never the private key.
        f.debug_struct("Identity")
            .field("device_id", &self.device_id)
            .field("public_key", &fingerprint(&self.public_key))
            .finish()
    }
}

impl Identity {
    /// Reads the identity in `data_dir`, or makes one if there's none or it's unreadable.
    pub fn load_or_create(data_dir: &Path) -> io::Result<Self> {
        let path = path(data_dir);
        if let Ok(text) = fs::read_to_string(&path)
            && let Some(identity) = parse(&text)
        {
            return Ok(identity);
        }
        let pair = noise::generate_keypair();
        let identity = Self {
            device_id: uuid::Uuid::now_v7().to_string(),
            private_key: pair.private,
            public_key: pair.public,
        };
        write_private(&path, &identity.to_text())?;
        Ok(identity)
    }

    /// The identity in `data_dir`, if sync has ever been on there. Never makes one.
    pub fn existing(data_dir: &Path) -> Option<Self> {
        parse(&fs::read_to_string(path(data_dir)).ok()?)
    }

    fn to_text(&self) -> String {
        format!(
            "kopi-sync-identity 1\n{}\n{}\n{}\n",
            self.device_id,
            hex(&self.private_key),
            hex(&self.public_key)
        )
    }
}

/// `<data dir>/sync/device.key`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("sync").join("device.key")
}

/// A short, stable label for a public key, for logs.
pub fn fingerprint(public_key: &[u8]) -> String {
    hex(&blake3::hash(public_key).as_bytes()[..6])
}

fn parse(text: &str) -> Option<Identity> {
    let mut lines = text.lines();
    if lines.next()? != "kopi-sync-identity 1" {
        return None;
    }
    let device_id = lines.next()?.to_owned();
    let private_key = unhex(lines.next()?)?;
    let public_key = unhex(lines.next()?)?;
    let valid = !device_id.is_empty()
        && device_id.len() <= 64
        && private_key.len() == 32
        && public_key.len() == 32;
    valid.then_some(Identity {
        device_id,
        private_key,
        public_key,
    })
}

/// Writes `text` to `path`, readable only by the user, replacing it whole.
fn write_private(path: &Path, text: &str) -> io::Result<()> {
    let dir = path.parent().expect("the key file has a folder");
    fs::create_dir_all(dir)?;
    let tmp = path.with_extension("tmp");
    {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        io::Write::write_all(&mut file, text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_made_once_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let first = Identity::load_or_create(dir.path()).unwrap();
        let again = Identity::load_or_create(dir.path()).unwrap();
        assert_eq!(first.device_id, again.device_id);
        assert_eq!(first.public_key, again.public_key);
        assert_eq!(first.private_key, again.private_key);
        assert_eq!(first.public_key.len(), 32);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(path(dir.path())).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert!(!format!("{first:?}").contains(&hex(&first.private_key)));
    }

    #[test]
    fn a_damaged_file_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("sync")).unwrap();
        fs::write(path(dir.path()), "garbage").unwrap();
        let identity = Identity::load_or_create(dir.path()).unwrap();
        assert_eq!(identity.private_key.len(), 32);
    }
}
