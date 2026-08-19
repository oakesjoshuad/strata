#![allow(clippy::disallowed_types)]

mod mutations;
mod queries;
mod validate;

use chrono::Utc;
use records::{Record, RecordId, ValidationError};
use refinery::embed_migrations;
use rusqlite::{Connection, Row, Transaction};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::path::Path;
use thiserror::Error;

embed_migrations!("migrations");

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[source] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(#[source] refinery::Error),
    #[error("validation error: {0}")]
    Validation(#[source] ValidationError),
    #[error("json error: {0}")]
    Json(#[source] serde_json::Error),
    #[error("record not found: {0}")]
    NotFound(RecordId),
    #[error("database validation failed")]
    InvalidDatabase(Vec<String>),
    #[error("invalid argument: {0}")]
    Argument(String),
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

impl From<ValidationError> for StoreError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

pub struct Store {
    conn: Connection,
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
        migrations::runner()
            .run(&mut conn)
            .map_err(StoreError::Migration)?;
        Ok(Self { conn })
    }
}

#[derive(Debug, Serialize)]
pub struct Graph {
    pub record: Record,
    pub relationships: Vec<records::Relationship>,
}

pub(crate) fn record_from_row(row: &Row<'_>) -> rusqlite::Result<Record> {
    let id = row.get(0)?;
    Ok(Record {
        id,
        title: row.get(1)?,
        status: row.get(2)?,
        document: serde_json::from_str(&row.get::<_, String>(3)?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
        })?,
        revision: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(crate) fn timestamp() -> String {
    Utc::now().to_rfc3339()
}

pub(crate) fn index_record(
    tx: &Transaction<'_>,
    id: &RecordId,
    title: &str,
    document: &JsonValue,
) -> Result<(), StoreError> {
    tx.execute(
        "DELETE FROM engineering_record_fts WHERE record_id = :id",
        rusqlite::named_params! { ":id": id },
    )?;
    tx.execute("INSERT INTO engineering_record_fts (record_id, title, body, tags) VALUES (:id, :title, :body, :tags)", rusqlite::named_params! { ":id": id, ":title": title, ":body": serde_json::to_string(document)?, ":tags": document.get("tags").map(ToString::to_string).unwrap_or_default() })?;
    Ok(())
}

pub(crate) fn insert_revision(
    tx: &Transaction<'_>,
    id: &RecordId,
    revision: u32,
    document: &JsonValue,
    summary: &str,
) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO record_revision (record_id, revision, document, changed_at, change_summary) VALUES (:id, :revision, :document, :changed_at, :summary)",
        rusqlite::named_params! {
            ":id": id,
            ":revision": revision,
            ":document": serde_json::to_string(document)?,
            ":changed_at": &timestamp(),
            ":summary": summary,
        },
    )?;
    Ok(())
}
