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
        tx.execute(
            "INSERT INTO code_reference (record_id, relation, path, symbol, line_start, line_end) VALUES (:record_id, :relation, :path, :symbol, :line_start, :line_end)",
            rusqlite::named_params! {
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
            record_id: record_id.clone(),
            relation: relation.into(),
            path: path.into(),
            symbol: symbol.map(Into::into),
            line_start,
            line_end,
        })
    }

    pub fn list_code_references(
        &self,
        record_id: &RecordId,
    ) -> Result<Vec<CodeReference>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT record_id, relation, path, symbol, line_start, line_end FROM code_reference WHERE record_id = :record_id ORDER BY path, line_start",
        )?;
        let rows = stmt.query_map(rusqlite::named_params! { ":record_id": record_id }, |row| {
            Ok(CodeReference {
                record_id: row.get(0)?,
                relation: row.get(1)?,
                path: row.get(2)?,
                symbol: row.get(3)?,
                line_start: row.get(4)?,
                line_end: row.get(5)?,
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
            "SELECT record_id, relation, path, symbol, line_start, line_end FROM code_reference ORDER BY record_id, path, line_start",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CodeReference {
                record_id: row.get(0)?,
                relation: row.get(1)?,
                path: row.get(2)?,
                symbol: row.get(3)?,
                line_start: row.get(4)?,
                line_end: row.get(5)?,
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
        assert_eq!(listed[0].path, "store/src/mutations.rs");
        assert_eq!(listed[0].symbol.as_deref(), Some("Store::revise"));
        assert_eq!(listed[0].line_start, Some(36));
        assert_eq!(listed[0].line_end, Some(53));
    }
}
