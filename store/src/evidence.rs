use crate::{Store, StoreError};
use records::{validate_evidence, Evidence, RecordId};
use serde_json::Value as JsonValue;

impl Store {
    pub fn add_evidence(
        &mut self,
        record_id: &RecordId,
        kind: &str,
        title: &str,
        uri: Option<&str>,
        content: Option<&str>,
        metadata: Option<&JsonValue>,
    ) -> Result<Evidence, StoreError> {
        self.get(record_id)?;
        validate_evidence(kind, title)?;
        let metadata_json = metadata.map(serde_json::to_string).transpose()?;
        let tx = self.conn.transaction()?;
        // Evidence ids are allocated per record, the same transactional
        // MAX+1-within-a-transaction pattern EDR-0001 uses for RecordId
        // numbers, just scoped to a record instead of a RecordKind.
        let ordinal: u32 = tx.query_row(
            "SELECT COUNT(*) + 1 FROM evidence WHERE record_id = :record_id",
            rusqlite::named_params! { ":record_id": record_id },
            |r| r.get(0),
        )?;
        let id = format!("{record_id}-EV-{ordinal:03}");
        tx.execute(
            "INSERT INTO evidence (id, record_id, kind, title, uri, content, metadata) VALUES (:id, :record_id, :kind, :title, :uri, :content, :metadata)",
            rusqlite::named_params! {
                ":id": &id,
                ":record_id": record_id,
                ":kind": kind,
                ":title": title,
                ":uri": uri,
                ":content": content,
                ":metadata": &metadata_json,
            },
        )?;
        tx.commit()?;
        Ok(Evidence {
            id,
            record_id: record_id.clone(),
            kind: kind.into(),
            title: title.into(),
            uri: uri.map(Into::into),
            content: content.map(Into::into),
            metadata: metadata.cloned(),
        })
    }

    pub fn list_evidence(&self, record_id: &RecordId) -> Result<Vec<Evidence>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, record_id, kind, title, uri, content, metadata FROM evidence WHERE record_id = :record_id ORDER BY id",
        )?;
        let rows = stmt.query_map(rusqlite::named_params! { ":record_id": record_id }, |row| {
            let metadata: Option<String> = row.get(6)?;
            let metadata = metadata
                .map(|m| serde_json::from_str(&m))
                .transpose()
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            Ok(Evidence {
                id: row.get(0)?,
                record_id: row.get(1)?,
                kind: row.get(2)?,
                title: row.get(3)?,
                uri: row.get(4)?,
                content: row.get(5)?,
                metadata,
            })
        })?;
        let mut evidence = Vec::new();
        for row in rows {
            evidence.push(row?);
        }
        Ok(evidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::{RecordKind, ValidationError};
    use serde_json::json;

    #[test]
    fn add_evidence_rejects_unknown_kind() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");

        assert!(matches!(
            store.add_evidence(&record.id, "vibes", "a title", None, None, None),
            Err(StoreError::Validation(
                ValidationError::InvalidEvidenceKind(_)
            ))
        ));
    }

    #[test]
    fn add_evidence_rejects_missing_record() {
        let mut store = Store::open_memory().expect("store");
        let missing = RecordId::new(RecordKind::Adr, 1);

        assert!(matches!(
            store.add_evidence(&missing, "benchmark", "a title", None, None, None),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn evidence_ids_are_allocated_per_record_starting_at_one() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");

        let first = store
            .add_evidence(
                &record.id,
                "benchmark",
                "first",
                Some("https://example.com"),
                None,
                Some(&json!({"n": 1})),
            )
            .expect("add first");
        let second = store
            .add_evidence(&record.id, "issue", "second", None, Some("body"), None)
            .expect("add second");

        assert_eq!(first.id, format!("{}-EV-001", record.id));
        assert_eq!(second.id, format!("{}-EV-002", record.id));

        let listed = store.list_evidence(&record.id).expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, first.id);
        assert_eq!(listed[0].metadata, Some(json!({"n": 1})));
        assert_eq!(listed[1].content.as_deref(), Some("body"));
    }
}
