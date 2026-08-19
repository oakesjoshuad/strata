#![allow(clippy::disallowed_types)]

use rusqlite::Connection;
use std::path::Path;

use crate::StoreError;

pub struct Store {
    pub(crate) conn: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        Self::configure(conn)
    }

    pub fn open_memory() -> Result<Self, StoreError> {
        Self::configure(Connection::open_in_memory()?)
    }

    fn configure(mut conn: Connection) -> Result<Self, StoreError> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        crate::migrations::runner()
            .run(&mut conn)
            .map_err(StoreError::Migration)?;
        Ok(Self { conn })
    }
}
