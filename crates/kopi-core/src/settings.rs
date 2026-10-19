//! Typed settings with their defaults, kept in the `settings` table. The history limits are in
//! [`crate::retention`]. See `docs/DATA_MODEL.md`.
//!
//! A missing or invalid value reads as its default, so a hand-edited or newer database never
//! stops Kopi from starting.

use std::str::FromStr;

use serde_json::Value;

use crate::privacy::KindSet;
use crate::storage::{ClipKind, StorageResult, Store};

const SHORTCUT: &str = "shortcut";
const CAPTURE_KINDS: &str = "capture.kinds";
const THEME: &str = "appearance.theme";
const ONBOARDED: &str = "onboarding.done";
const AUTO_UPDATE: &str = "updates.automatic";
const SYNC_ENABLED: &str = "sync.enabled";
const SYNC_KINDS: &str = "sync.kinds";
const SYNC_DEVICE_NAME: &str = "sync.device_name";

/// The kinds that can sync. File clips are paths, which mean nothing on another computer.
pub const SYNCABLE_KINDS: KindSet = KindSet::ALL.without(ClipKind::File);

/// The app's color scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follows the system's light or dark mode.
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

impl Store {
    /// The shortcut that opens the panel, as the global-shortcut plugin writes it
    /// (`shift+super+KeyV`), or none for the default.
    pub fn shortcut(&self) -> StorageResult<Option<String>> {
        Ok(self
            .setting(SHORTCUT)?
            .and_then(|v| v.as_str().map(str::to_owned))
            .filter(|s| !s.is_empty()))
    }

    pub fn set_shortcut(&self, shortcut: &str) -> StorageResult<()> {
        self.set_settings(&[(SHORTCUT, Value::from(shortcut))])
    }

    /// The kinds to capture. All of them by default. Names Kopi doesn't know are ignored.
    pub fn capture_kinds(&self) -> StorageResult<KindSet> {
        let Some(Value::Array(names)) = self.setting(CAPTURE_KINDS)? else {
            return Ok(KindSet::ALL);
        };
        Ok(names
            .iter()
            .filter_map(|name| ClipKind::from_str(name.as_str()?).ok())
            .collect())
    }

    pub fn set_capture_kinds(&self, kinds: KindSet) -> StorageResult<()> {
        let names = kinds.iter().map(|k| Value::from(k.as_str())).collect();
        self.set_settings(&[(CAPTURE_KINDS, Value::Array(names))])
    }

    pub fn theme(&self) -> StorageResult<Theme> {
        Ok(self
            .setting(THEME)?
            .and_then(|v| v.as_str().and_then(Theme::parse))
            .unwrap_or_default())
    }

    pub fn set_theme(&self, theme: Theme) -> StorageResult<()> {
        self.set_settings(&[(THEME, Value::from(theme.as_str()))])
    }

    /// Whether the user has finished or closed onboarding. Until then it opens at launch.
    pub fn onboarded(&self) -> StorageResult<bool> {
        Ok(self
            .setting(ONBOARDED)?
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    pub fn set_onboarded(&self, done: bool) -> StorageResult<()> {
        self.set_settings(&[(ONBOARDED, Value::from(done))])
    }

    /// Whether Kopi checks for updates by itself and installs them. On by default.
    pub fn auto_update(&self) -> StorageResult<bool> {
        Ok(self
            .setting(AUTO_UPDATE)?
            .and_then(|v| v.as_bool())
            .unwrap_or(true))
    }

    pub fn set_auto_update(&self, enabled: bool) -> StorageResult<()> {
        self.set_settings(&[(AUTO_UPDATE, Value::from(enabled))])
    }

    /// Whether sync with the user's other computers is on. Off by default.
    pub fn sync_enabled(&self) -> StorageResult<bool> {
        Ok(self
            .setting(SYNC_ENABLED)?
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    pub fn set_sync_enabled(&self, enabled: bool) -> StorageResult<()> {
        self.set_settings(&[(SYNC_ENABLED, Value::from(enabled))])
    }

    /// The kinds to send to and take from other computers. Every kind but files by default.
    pub fn sync_kinds(&self) -> StorageResult<KindSet> {
        let Some(Value::Array(names)) = self.setting(SYNC_KINDS)? else {
            return Ok(SYNCABLE_KINDS);
        };
        Ok(names
            .iter()
            .filter_map(|name| ClipKind::from_str(name.as_str()?).ok())
            .filter(|kind| SYNCABLE_KINDS.contains(*kind))
            .collect())
    }

    /// The name the user gave this computer for sync, if they changed it.
    pub fn sync_device_name(&self) -> StorageResult<Option<String>> {
        Ok(self
            .setting(SYNC_DEVICE_NAME)?
            .and_then(|v| v.as_str().map(str::to_owned))
            .filter(|s| !s.trim().is_empty()))
    }

    /// Saves this computer's name, trimmed. Fails if it's empty or too long.
    pub fn set_sync_device_name(&self, name: &str) -> StorageResult<String> {
        let name = crate::storage::sync::check_device_name(name)?;
        self.set_settings(&[(SYNC_DEVICE_NAME, Value::from(name.as_str()))])?;
        Ok(name)
    }

    pub fn set_sync_kinds(&self, kinds: KindSet) -> StorageResult<()> {
        let names = kinds
            .iter()
            .filter(|kind| SYNCABLE_KINDS.contains(*kind))
            .map(|k| Value::from(k.as_str()))
            .collect();
        self.set_settings(&[(SYNC_KINDS, Value::Array(names))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn defaults() {
        let (_dir, store) = store();
        assert_eq!(store.shortcut().unwrap(), None);
        assert_eq!(store.capture_kinds().unwrap(), KindSet::ALL);
        assert_eq!(store.theme().unwrap(), Theme::System);
        assert!(!store.onboarded().unwrap());
        assert!(store.auto_update().unwrap());
        assert!(!store.sync_enabled().unwrap());
        assert_eq!(store.sync_kinds().unwrap(), SYNCABLE_KINDS);
    }

    #[test]
    fn round_trips() {
        let (_dir, store) = store();
        store.set_shortcut("alt+super+KeyC").unwrap();
        assert_eq!(store.shortcut().unwrap().as_deref(), Some("alt+super+KeyC"));

        let kinds = KindSet::ALL
            .without(ClipKind::Image)
            .without(ClipKind::File);
        store.set_capture_kinds(kinds).unwrap();
        assert_eq!(store.capture_kinds().unwrap(), kinds);
        store.set_capture_kinds(KindSet::NONE).unwrap();
        assert_eq!(store.capture_kinds().unwrap(), KindSet::NONE);

        for theme in [Theme::Light, Theme::Dark, Theme::System] {
            store.set_theme(theme).unwrap();
            assert_eq!(store.theme().unwrap(), theme);
        }

        store.set_onboarded(true).unwrap();
        assert!(store.onboarded().unwrap());

        store.set_auto_update(false).unwrap();
        assert!(!store.auto_update().unwrap());

        store.set_sync_enabled(true).unwrap();
        assert!(store.sync_enabled().unwrap());
        // Files never sync, even if asked.
        store
            .set_sync_kinds(KindSet::NONE.with(ClipKind::Text).with(ClipKind::File))
            .unwrap();
        assert_eq!(
            store.sync_kinds().unwrap(),
            KindSet::NONE.with(ClipKind::Text)
        );
    }

    #[test]
    fn invalid_values_read_as_defaults() {
        let (_dir, store) = store();
        store
            .set_settings(&[
                (SHORTCUT, Value::from(3)),
                (CAPTURE_KINDS, Value::from("text")),
                (THEME, Value::from("sepia")),
                (ONBOARDED, Value::from("yes")),
                (AUTO_UPDATE, Value::from("no")),
            ])
            .unwrap();
        assert_eq!(store.shortcut().unwrap(), None);
        assert_eq!(store.capture_kinds().unwrap(), KindSet::ALL);
        assert_eq!(store.theme().unwrap(), Theme::System);
        assert!(!store.onboarded().unwrap());
        assert!(store.auto_update().unwrap());

        store
            .set_settings(&[(CAPTURE_KINDS, serde_json::json!(["link", "hologram", 7]))])
            .unwrap();
        assert_eq!(
            store.capture_kinds().unwrap(),
            KindSet::NONE.with(ClipKind::Link)
        );
    }
}
