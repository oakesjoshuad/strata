#![allow(clippy::disallowed_types)]

use chrono::Utc;
use refinery::embed_migrations;
use rusqlite::{Connection, OptionalExtension, Row};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::path::Path;
use strata_records::{
    validate_document, validate_relationship, validate_transition, Record, RecordId, RecordKind,
    Relationship, Revision, Status, ValidationError, RELATIONSHIPS,
};
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

    pub fn create(
        &mut self,
        kind: RecordKind,
        title: &str,
        document: JsonValue,
    ) -> Result<Record, StoreError> {
        if title.trim().is_empty() {
            return Err(ValidationError::EmptyTitle.into());
        }
        validate_document(kind, &document)?;
        let tx = self.conn.transaction()?;
        let number: u32 = tx.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM engineering_record WHERE kind = :kind",
            rusqlite::named_params! { ":kind": kind },
            |r| r.get(0),
        )?;
        let id = RecordId::new(kind, number);
        let now = timestamp();
        let doc = serde_json::to_string(&document)?;
        tx.execute("INSERT INTO engineering_record (id, kind, number, title, status, document, revision, created_at, updated_at) VALUES (:id, :kind, :number, :title, :status, :document, :revision, :created_at, :updated_at)", rusqlite::named_params! { ":id": id, ":kind": kind, ":number": number, ":title": title, ":status": kind.initial_status(), ":document": doc, ":revision": 1, ":created_at": &now, ":updated_at": &now })?;
        tx.execute("INSERT INTO record_revision (record_id, revision, document, changed_at, change_summary) VALUES (:id, :revision, :document, :changed_at, :summary)", rusqlite::named_params! { ":id": id, ":revision": 1, ":document": serde_json::to_string(&document)?, ":changed_at": &now, ":summary": "created" })?;
        index_record(&tx, &id, title, &document)?;
        tx.commit()?;
        self.get(&id)
    }

    pub fn get(&self, id: &RecordId) -> Result<Record, StoreError> {
        self.conn.query_row("SELECT id, title, status, document, revision, created_at, updated_at FROM engineering_record WHERE id = :id", rusqlite::named_params! { ":id": id }, record_from_row).optional()?.ok_or_else(|| StoreError::NotFound(id.clone()))
    }
    pub fn revise(
        &mut self,
        id: &RecordId,
        document: JsonValue,
        summary: Option<&str>,
    ) -> Result<Record, StoreError> {
        let current = self.get(id)?;
        validate_document(id.kind, &document)?;
        let tx = self.conn.transaction()?;
        let revision = current.revision + 1;
        let now = timestamp();
        let doc = serde_json::to_string(&document)?;
        tx.execute("UPDATE engineering_record SET document = :document, revision = :revision, updated_at = :updated_at WHERE id = :id", rusqlite::named_params! { ":document": &doc, ":revision": revision, ":updated_at": &now, ":id": id })?;
        tx.execute("INSERT INTO record_revision (record_id, revision, document, changed_at, change_summary) VALUES (:id, :revision, :document, :changed_at, :summary)", rusqlite::named_params! { ":id": id, ":revision": revision, ":document": &doc, ":changed_at": &now, ":summary": summary.unwrap_or("revised") })?;
        index_record(&tx, id, &current.title, &document)?;
        tx.commit()?;
        self.get(id)
    }
    pub fn set_status(&mut self, id: &RecordId, status: Status) -> Result<Record, StoreError> {
        let current = self.get(id)?;
        validate_transition(id.kind, current.status, status)?;
        let tx = self.conn.transaction()?;
        let revision = current.revision + 1;
        let now = timestamp();
        let doc = serde_json::to_string(&current.document)?;
        tx.execute("UPDATE engineering_record SET status = :status, revision = :revision, updated_at = :updated_at WHERE id = :id", rusqlite::named_params! { ":status": status, ":revision": revision, ":updated_at": &now, ":id": id })?;
        tx.execute("INSERT INTO record_revision (record_id, revision, document, changed_at, change_summary) VALUES (:id, :revision, :document, :changed_at, :summary)", rusqlite::named_params! { ":id": id, ":revision": revision, ":document": &doc, ":changed_at": &now, ":summary": format!("status changed to {status}") })?;
        tx.commit()?;
        self.get(id)
    }
    pub fn link(
        &mut self,
        source: &RecordId,
        relation: &str,
        target: &RecordId,
    ) -> Result<Relationship, StoreError> {
        validate_relationship(source, relation, target)?;
        if self.get(source).is_err() {
            return Err(StoreError::NotFound(source.clone()));
        }
        if self.get(target).is_err() {
            return Err(StoreError::NotFound(target.clone()));
        }
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO record_relation (source_id, relation, target_id) VALUES (:source, :relation, :target)", rusqlite::named_params! { ":source": source, ":relation": relation, ":target": target })?;
        tx.commit()?;
        Ok(Relationship {
            source_id: source.clone(),
            relation: relation.into(),
            target_id: target.clone(),
        })
    }
    pub fn history(&self, id: &RecordId) -> Result<Vec<Revision>, StoreError> {
        let mut stmt = self.conn.prepare("SELECT record_id, revision, document, changed_at, changed_by, change_summary FROM record_revision WHERE record_id = :id ORDER BY revision")?;
        let rows = stmt.query_map(rusqlite::named_params! { ":id": id }, |row| {
            Ok(Revision {
                record_id: row.get(0)?,
                revision: row.get(1)?,
                document: serde_json::from_str(&row.get::<_, String>(2)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                changed_at: row.get(3)?,
                changed_by: row.get(4)?,
                change_summary: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn search(&self, query: &str, limit: u32, offset: u32) -> Result<Vec<Record>, StoreError> {
        let mut where_builder = WhereBuilder::new();
        where_builder.push("engineering_record_fts MATCH :query", ":query", query);
        let sql = format!("SELECT r.id, r.title, r.status, r.document, r.revision, r.created_at, r.updated_at FROM engineering_record r JOIN engineering_record_fts ON engineering_record_fts.record_id = r.id WHERE {} ORDER BY rank LIMIT :limit OFFSET :offset", where_builder.sql());
        let mut stmt = self.conn.prepare(&sql)?;
        for (name, value) in where_builder.bindings() {
            stmt.raw_bind_parameter(name.as_str(), value)?;
        }
        stmt.raw_bind_parameter(":limit", limit)?;
        stmt.raw_bind_parameter(":offset", offset)?;
        let mut rows = stmt.raw_query();
        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            records.push(record_from_row(row)?);
        }
        Ok(records)
    }
    pub fn graph(&self, id: &RecordId) -> Result<Graph, StoreError> {
        let record = self.get(id)?;
        let mut stmt = self.conn.prepare("SELECT source_id, relation, target_id FROM record_relation WHERE source_id = :id OR target_id = :id ORDER BY source_id, relation, target_id")?;
        let rows = stmt.query_map(rusqlite::named_params! { ":id": id }, |r| {
            Ok(Relationship {
                source_id: r.get(0)?,
                relation: r.get(1)?,
                target_id: r.get(2)?,
            })
        })?;
        Ok(Graph {
            record,
            relationships: rows.collect::<Result<Vec<_>, _>>()?,
        })
    }
    pub fn validate(&self) -> Result<Vec<String>, StoreError> {
        let mut issues = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT id, kind, document FROM engineering_record")?;
        let rows = stmt.query_map([], |r| {
            let id: RecordId = r.get(0)?;
            let kind: RecordKind = r.get(1)?;
            let doc: String = r.get(2)?;
            Ok((id, kind, doc))
        })?;
        for row in rows {
            let (id, kind, doc) = row?;
            match serde_json::from_str::<JsonValue>(&doc)
                .map_err(StoreError::Json)
                .and_then(|d| validate_document(kind, &d).map_err(StoreError::Validation))
            {
                Ok(()) => {}
                Err(e) => issues.push(format!("{id}: {e}")),
            }
        }
        let mut dup = self.conn.prepare("SELECT kind, number, COUNT(*) FROM engineering_record GROUP BY kind, number HAVING COUNT(*) > 1")?;
        for row in dup.query_map([], |r| {
            Ok(format!(
                "duplicate identifier: {}-{:04}",
                r.get::<_, String>(0)?,
                r.get::<_, u32>(1)?
            ))
        })? {
            issues.push(row?);
        }
        let mut rel = self.conn.prepare("SELECT rr.source_id, rr.relation, rr.target_id FROM record_relation rr LEFT JOIN engineering_record s ON s.id = rr.source_id LEFT JOIN engineering_record t ON t.id = rr.target_id WHERE s.id IS NULL OR t.id IS NULL OR rr.relation NOT IN ('relates-to','derived-from','explored-by','produces','resolves','constrains','implements','implemented-by','supported-by','supersedes')")?;
        for row in rel.query_map([], |r| {
            Ok(format!(
                "broken or invalid relationship: {} {} {}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?
            ))
        })? {
            issues.push(row?);
        }
        let mut sup = self.conn.prepare("SELECT rr.source_id, rr.target_id FROM record_relation rr JOIN engineering_record t ON t.id = rr.target_id WHERE rr.relation = 'supersedes' AND t.status = 'accepted'")?;
        for row in sup.query_map([], |r| {
            Ok(format!(
                "{} supersedes {} while target remains accepted",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?
            ))
        })? {
            issues.push(row?);
        }
        Ok(issues)
    }
    pub fn relationships(&self) -> &'static [&'static str] {
        &RELATIONSHIPS
    }
}

#[derive(Debug, Serialize)]
pub struct Graph {
    pub record: Record,
    pub relationships: Vec<Relationship>,
}
pub struct WhereBuilder {
    clauses: Vec<String>,
    bindings: Vec<(String, String)>,
}
impl WhereBuilder {
    pub fn new() -> Self {
        Self {
            clauses: Vec::new(),
            bindings: Vec::new(),
        }
    }
    pub fn push(&mut self, clause: &str, name: &str, value: &str) {
        self.clauses.push(clause.into());
        self.bindings.push((name.into(), value.into()));
    }
    fn sql(&self) -> String {
        self.clauses.join(" AND ")
    }
    fn bindings(&self) -> &[(String, String)] {
        &self.bindings
    }
}

impl Default for WhereBuilder {
    fn default() -> Self {
        Self::new()
    }
}
fn record_from_row(row: &Row<'_>) -> rusqlite::Result<Record> {
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
fn timestamp() -> String {
    Utc::now().to_rfc3339()
}
fn index_record(
    tx: &rusqlite::Transaction<'_>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_have_valid_creation_documents() {
        let mut store = Store::open_memory().expect("store");
        for kind in RecordKind::ALL {
            let record = store
                .create(kind, kind.code(), kind.default_document(kind.code()))
                .expect("create");
            assert_eq!(record.id.kind, kind);
            assert_eq!(record.revision, 1);
        }
    }
    #[test]
    fn lifecycle_revision_and_search_are_transactional() {
        let mut s = Store::open_memory().expect("store");
        let r = s
            .create(
                RecordKind::Adr,
                "SQLite",
                RecordKind::Adr.default_document("SQLite"),
            )
            .expect("create");
        s.revise(
            &r.id,
            RecordKind::Adr.default_document("canonical SQLite persistence"),
            None,
        )
        .expect("revise");
        assert_eq!(s.history(&r.id).expect("history").len(), 2);
        assert_eq!(s.search("persist", 10, 0).expect("search").len(), 1);
    }
    #[test]
    fn invalid_transition_does_not_change_record() {
        let mut s = Store::open_memory().expect("store");
        let r = s
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");
        assert!(s.set_status(&r.id, Status::Superseded).is_err());
        assert_eq!(s.get(&r.id).expect("get").status, Status::Proposed);
    }

    #[test]
    fn supersedes_accepted_target_is_reported() {
        let mut store = Store::open_memory().expect("store");
        let target = store
            .create(
                RecordKind::Adr,
                "target",
                RecordKind::Adr.default_document("target"),
            )
            .expect("create target");
        store
            .set_status(&target.id, Status::Accepted)
            .expect("accept target");
        let source = store
            .create(
                RecordKind::Adr,
                "source",
                RecordKind::Adr.default_document("source"),
            )
            .expect("create source");
        store
            .link(&source.id, "supersedes", &target.id)
            .expect("link");
        let issues = store.validate().expect("validate");
        assert!(issues
            .iter()
            .any(|issue| issue.contains("remains accepted")));
    }
}
