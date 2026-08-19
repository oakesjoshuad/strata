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
        backfill_slugs(&mut conn)?;
        Ok(Self { conn })
    }
}

fn backfill_slugs(conn: &mut Connection) -> Result<(), StoreError> {
    let mut stmt =
        conn.prepare("SELECT id, title FROM engineering_record WHERE slug = :empty ORDER BY id")?;
    let records = stmt
        .query_map(rusqlite::named_params! { ":empty": "" }, |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    for (id, title) in records {
        let slug = crate::slug::slugify(&title);
        conn.execute(
            "UPDATE engineering_record SET slug = :slug WHERE id = :id AND slug = :empty",
            rusqlite::named_params! { ":slug": slug, ":id": id, ":empty": "" },
        )?;
    }
    Ok(())
}
