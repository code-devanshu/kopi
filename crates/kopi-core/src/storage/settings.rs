//! The `settings` table: one JSON value per key. The keys, their defaults and their checks live
//! with the features that use them, such as the retention limits.

use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::Value;

use super::{StorageResult, Store, now_ms};

impl Store {
    /// The stored value for `key`, or none if it was never set. A value that isn't valid JSON
    /// reads as none, so the caller falls back to its default.
    pub fn setting(&self, key: &str) -> StorageResult<Option<Value>> {
        let text: Option<String> = self.with_reader(|conn| {
            Ok(conn
                .prepare_cached("SELECT value FROM settings WHERE key = ?1")?
                .query_row([key], |row| row.get(0))
                .optional()?)
        })?;
        Ok(text.and_then(|text| serde_json::from_str(&text).ok()))
    }

    /// Stores several values together, in one transaction.
    pub fn set_settings(&self, values: &[(&str, Value)]) -> StorageResult<()> {
        let now = now_ms();
        let mut conn = self.writer();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        {
            let mut upsert = tx.prepare_cached(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                 WHERE value IS NOT excluded.value",
            )?;
            for (key, value) in values {
                upsert.execute(params![key, value.to_string(), now])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}
