use crate::{Store, StoreError};
use records::{validate_code_reference, CodeReference, RecordId};

impl Store {
    pub fn add_code_reference(
        &mut self,
        record_id: &RecordId,
        relation: &str,
        path: &str,
        symbol: Option<&str>,
        line_start: Option<u32>,
        line_end: Option<u32>,
    ) -> Result<CodeReference, StoreError> {
        self.get(record_id)?;
        validate_code_reference(relation, path)?;
        let tx = self.conn.transaction()?;
        // Ordinal allocation must survive deletion without colliding: unlike
        // evidence (add-only today), code references can be removed, so a
        // plain COUNT(*)+1 would reissue an already-used ordinal the moment
        // an earlier reference is deleted. MAX-based allocation, extracting
        // the zero-padded ordinal from the highest existing id for this
        // record, only ever moves forward -- the same gap-leaving guarantee
        // EDR-0006 already anticipated for evidence deletion, applied here
        // because code references actually support it.
        let ordinal: u32 = tx.query_row(
            "SELECT COALESCE(MAX(CAST(SUBSTR(id, -3) AS INTEGER)), 0) + 1 FROM code_reference WHERE record_id = :record_id",
            rusqlite::named_params! { ":record_id": record_id },
            |r| r.get(0),
        )?;
        let id = format!("{record_id}-CR-{ordinal:03}");
        tx.execute(
            "INSERT INTO code_reference (id, record_id, relation, path, symbol, line_start, line_end) VALUES (:id, :record_id, :relation, :path, :symbol, :line_start, :line_end)",
            rusqlite::named_params! {
                ":id": &id,
                ":record_id": record_id,
                ":relation": relation,
                ":path": path,
                ":symbol": symbol,
                ":line_start": line_start,
                ":line_end": line_end,
            },
        )?;
        tx.commit()?;
        Ok(CodeReference {
            id,
            record_id: record_id.clone(),
            relation: relation.into(),
            path: path.into(),
            symbol: symbol.map(Into::into),
            line_start,
            line_end,
        })
    }

    pub fn update_code_reference(
        &mut self,
        id: &str,
        relation: &str,
        path: &str,
        symbol: Option<&str>,
        line_start: Option<u32>,
        line_end: Option<u32>,
    ) -> Result<CodeReference, StoreError> {
        validate_code_reference(relation, path)?;
        let record_id: RecordId = self
            .conn
            .query_row(
                "SELECT record_id FROM code_reference WHERE id = :id",
                rusqlite::named_params! { ":id": id },
                |r| r.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::CodeReferenceNotFound(id.to_string())
                }
                other => StoreError::from(other),
            })?;
        self.conn.execute(
            "UPDATE code_reference SET relation = :relation, path = :path, symbol = :symbol, line_start = :line_start, line_end = :line_end WHERE id = :id",
            rusqlite::named_params! {
                ":id": id,
                ":relation": relation,
                ":path": path,
                ":symbol": symbol,
                ":line_start": line_start,
                ":line_end": line_end,
            },
        )?;
        Ok(CodeReference {
            id: id.to_string(),
            record_id,
            relation: relation.into(),
            path: path.into(),
            symbol: symbol.map(Into::into),
            line_start,
            line_end,
        })
    }

    pub fn remove_code_reference(&mut self, id: &str) -> Result<(), StoreError> {
        let removed = self.conn.execute(
            "DELETE FROM code_reference WHERE id = :id",
            rusqlite::named_params! { ":id": id },
        )?;
        if removed == 0 {
            return Err(StoreError::CodeReferenceNotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn list_code_references(
        &self,
        record_id: &RecordId,
    ) -> Result<Vec<CodeReference>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, record_id, relation, path, symbol, line_start, line_end FROM code_reference WHERE record_id = :record_id ORDER BY path, line_start",
        )?;
        let rows = stmt.query_map(rusqlite::named_params! { ":record_id": record_id }, |row| {
            Ok(CodeReference {
                id: row.get(0)?,
                record_id: row.get(1)?,
                relation: row.get(2)?,
                path: row.get(3)?,
                symbol: row.get(4)?,
                line_start: row.get(5)?,
                line_end: row.get(6)?,
            })
        })?;
        let mut refs = Vec::new();
        for row in rows {
            refs.push(row?);
        }
        Ok(refs)
    }

    pub fn list_all_code_references(&self) -> Result<Vec<CodeReference>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, record_id, relation, path, symbol, line_start, line_end FROM code_reference ORDER BY record_id, path, line_start",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CodeReference {
                id: row.get(0)?,
                record_id: row.get(1)?,
                relation: row.get(2)?,
                path: row.get(3)?,
                symbol: row.get(4)?,
                line_start: row.get(5)?,
                line_end: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::{RecordKind, ValidationError};

    #[test]
    fn add_code_reference_rejects_unknown_relation() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");

        assert!(matches!(
            store.add_code_reference(&record.id, "vibes", "src/lib.rs", None, None, None),
            Err(StoreError::Validation(
                ValidationError::InvalidRelationship(_)
            ))
        ));
    }

    #[test]
    fn add_code_reference_rejects_missing_record() {
        let mut store = Store::open_memory().expect("store");
        let missing = RecordId::new(RecordKind::Adr, 1);

        assert!(matches!(
            store.add_code_reference(&missing, "constrains", "src/lib.rs", None, None, None),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn code_references_round_trip_through_list() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");

        store
            .add_code_reference(
                &record.id,
                "constrains",
                "store/src/mutations.rs",
                Some("Store::revise"),
                Some(36),
                Some(53),
            )
            .expect("add");

        let listed = store.list_code_references(&record.id).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, format!("{}-CR-001", record.id));
        assert_eq!(listed[0].path, "store/src/mutations.rs");
        assert_eq!(listed[0].symbol.as_deref(), Some("Store::revise"));
        assert_eq!(listed[0].line_start, Some(36));
        assert_eq!(listed[0].line_end, Some(53));
    }

    #[test]
    fn code_reference_ids_are_allocated_per_record_starting_at_one() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");

        let first = store
            .add_code_reference(&record.id, "constrains", "a.rs", None, None, None)
            .expect("add first");
        let second = store
            .add_code_reference(&record.id, "constrains", "b.rs", None, None, None)
            .expect("add second");

        assert_eq!(first.id, format!("{}-CR-001", record.id));
        assert_eq!(second.id, format!("{}-CR-002", record.id));
    }

    #[test]
    fn update_code_reference_changes_fields_and_persists() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");
        let created = store
            .add_code_reference(&record.id, "constrains", "a.rs", None, None, None)
            .expect("add");

        let updated = store
            .update_code_reference(
                &created.id,
                "implements",
                "b.rs",
                Some("Store::update_code_reference"),
                Some(10),
                Some(20),
            )
            .expect("update");

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.relation, "implements");
        assert_eq!(updated.path, "b.rs");
        assert_eq!(
            updated.symbol.as_deref(),
            Some("Store::update_code_reference")
        );

        let listed = store.list_code_references(&record.id).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].relation, "implements");
        assert_eq!(listed[0].path, "b.rs");
    }

    #[test]
    fn update_code_reference_rejects_unknown_id() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");
        let missing_id = format!("{}-CR-999", record.id);

        assert!(matches!(
            store.update_code_reference(&missing_id, "constrains", "a.rs", None, None, None),
            Err(StoreError::CodeReferenceNotFound(id)) if id == missing_id
        ));
    }

    #[test]
    fn update_code_reference_rejects_unknown_relation() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");
        let created = store
            .add_code_reference(&record.id, "constrains", "a.rs", None, None, None)
            .expect("add");

        assert!(matches!(
            store.update_code_reference(&created.id, "vibes", "a.rs", None, None, None),
            Err(StoreError::Validation(
                ValidationError::InvalidRelationship(_)
            ))
        ));
    }

    #[test]
    fn remove_code_reference_deletes_and_rejects_unknown_id() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");
        let created = store
            .add_code_reference(&record.id, "constrains", "a.rs", None, None, None)
            .expect("add");

        store.remove_code_reference(&created.id).expect("remove");
        assert!(store
            .list_code_references(&record.id)
            .expect("list")
            .is_empty());

        assert!(matches!(
            store.remove_code_reference(&created.id),
            Err(StoreError::CodeReferenceNotFound(id)) if id == created.id
        ));
    }

    #[test]
    fn ordinal_allocation_never_collides_after_a_delete() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");

        let first = store
            .add_code_reference(&record.id, "constrains", "a.rs", None, None, None)
            .expect("add first");
        let second = store
            .add_code_reference(&record.id, "constrains", "b.rs", None, None, None)
            .expect("add second");
        assert_eq!(first.id, format!("{}-CR-001", record.id));
        assert_eq!(second.id, format!("{}-CR-002", record.id));

        // Deleting the first (lower-ordinal) reference must not free its
        // ordinal for reuse -- a naive COUNT(*)+1 scheme would reissue
        // CR-002 here, colliding with the still-live second reference.
        store.remove_code_reference(&first.id).expect("remove");
        let third = store
            .add_code_reference(&record.id, "constrains", "c.rs", None, None, None)
            .expect("add third");
        assert_eq!(third.id, format!("{}-CR-003", record.id));
        assert_ne!(third.id, second.id);
    }
}
