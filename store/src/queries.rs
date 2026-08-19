use crate::{record_from_row, Graph, Store, StoreError};
use kernel::WhereBuilder;
use records::{Record, RecordId, Relationship, Revision, RELATIONSHIPS};
use rusqlite::OptionalExtension;

impl Store {
    pub fn get(&self, id: &RecordId) -> Result<Record, StoreError> {
        self.conn
            .query_row(
                "SELECT id, title, status, document, revision, created_at, updated_at FROM engineering_record WHERE id = :id",
                rusqlite::named_params! { ":id": id },
                record_from_row,
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound(id.clone()))
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
        where_builder.push(
            "engineering_record_fts MATCH :query",
            ":query",
            rusqlite::types::Value::Text(query.into()),
        );
        where_builder.push_binding(":limit", rusqlite::types::Value::Integer(i64::from(limit)));
        where_builder.push_binding(
            ":offset",
            rusqlite::types::Value::Integer(i64::from(offset)),
        );
        let sql = format!("SELECT r.id, r.title, r.status, r.document, r.revision, r.created_at, r.updated_at FROM engineering_record r JOIN engineering_record_fts ON engineering_record_fts.record_id = r.id WHERE {} ORDER BY rank LIMIT :limit OFFSET :offset", where_builder.sql());
        let mut stmt = self.conn.prepare(&sql)?;
        for (name, value) in where_builder.bindings() {
            stmt.raw_bind_parameter(name.as_str(), value)?;
        }
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

    pub fn relationships(&self) -> &'static [&'static str] {
        &RELATIONSHIPS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::RecordKind;

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
}
