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
        backfill_code_reference_ids(&mut conn)?;
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

// Rows created before V4__add_code_reference_id.sql have no id. Backfill them
// with the same {record_id}-CR-{ordinal:03} scheme EDR-0006 established for
// evidence, ordered by rowid so the assigned ordinal matches original
// insertion order within each record.
fn backfill_code_reference_ids(conn: &mut Connection) -> Result<(), StoreError> {
    let mut stmt = conn.prepare(
        "SELECT rowid, record_id FROM code_reference WHERE id = :empty ORDER BY record_id, rowid",
    )?;
    let rows = stmt
        .query_map(rusqlite::named_params! { ":empty": "" }, |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let mut ordinal_by_record: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    for (rowid, record_id) in rows {
        let ordinal = ordinal_by_record.entry(record_id.clone()).or_insert(0);
        *ordinal += 1;
        let id = format!("{record_id}-CR-{ordinal:03}");
        conn.execute(
            "UPDATE code_reference SET id = :id WHERE rowid = :rowid",
            rusqlite::named_params! { ":id": id, ":rowid": rowid },
        )?;
    }
    Ok(())
}
