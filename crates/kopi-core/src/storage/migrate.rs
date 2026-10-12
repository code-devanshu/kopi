//! Applies the SQL files in `migrations/` in order, tracked with `PRAGMA user_version`.

use rusqlite::{Connection, TransactionBehavior};

use super::{StorageError, StorageResult};

/// Every migration, oldest first. Migration `i` takes the schema from version `i` to `i + 1`.
/// Never edit one that has shipped. Add a new file instead.
const MIGRATIONS: &[&str] = &[include_str!("../../migrations/0001_init.sql")];

/// The schema version this build writes.
pub const SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

/// Brings the database up to [`SCHEMA_VERSION`]. Each migration runs in its own transaction
/// together with its version bump, so a failure leaves the database at the last good version.
pub(crate) fn migrate(conn: &mut Connection) -> StorageResult<()> {
    let found = user_version(conn)?;
    if found > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema {
            found,
            supported: SCHEMA_VERSION,
        });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(found as usize) {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as u32 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

pub(crate) fn user_version(conn: &Connection) -> rusqlite::Result<u32> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
}
