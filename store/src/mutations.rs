use crate::{index_record, insert_revision, slug, timestamp, Store, StoreError};
use records::{
    validate_document, validate_relationship, validate_transition, Record, RecordId, RecordKind,
    Relationship, Status, ValidationError,
};
use serde_json::Value as JsonValue;

impl Store {
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
        let slug = slug::slugify(title);
        let now = timestamp();
        let doc = serde_json::to_string(&document)?;
        tx.execute("INSERT INTO engineering_record (id, kind, number, title, slug, status, document, revision, created_at, updated_at) VALUES (:id, :kind, :number, :title, :slug, :status, :document, :revision, :created_at, :updated_at)", rusqlite::named_params! { ":id": id, ":kind": kind, ":number": number, ":title": title, ":slug": slug, ":status": kind.initial_status(), ":document": doc, ":revision": 1, ":created_at": &now, ":updated_at": &now })?;
        insert_revision(&tx, &id, 1, &document, "created")?;
        index_record(&tx, &id, title, &document)?;
        tx.commit()?;
        self.get(&id)
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
        insert_revision(&tx, id, revision, &document, summary.unwrap_or("revised"))?;
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
        tx.execute("UPDATE engineering_record SET status = :status, revision = :revision, updated_at = :updated_at WHERE id = :id", rusqlite::named_params! { ":status": status, ":revision": revision, ":updated_at": &now, ":id": id })?;
        let summary = format!("status changed to {status}");
        insert_revision(&tx, id, revision, &current.document, &summary)?;
        tx.commit()?;
        self.get(id)
    }

    pub fn retitle(&mut self, id: &RecordId, title: &str) -> Result<Record, StoreError> {
        if title.trim().is_empty() {
            return Err(ValidationError::EmptyTitle.into());
        }
        let current = self.get(id)?;
        let tx = self.conn.transaction()?;
        let revision = current.revision + 1;
        let now = timestamp();
        tx.execute(
            "UPDATE engineering_record SET title = :title, revision = :revision, updated_at = :updated_at WHERE id = :id",
            rusqlite::named_params! {
                ":title": title,
                ":revision": revision,
                ":updated_at": &now,
                ":id": id,
            },
        )?;
        let summary = format!("title changed to '{title}'");
        insert_revision(&tx, id, revision, &current.document, &summary)?;
        index_record(&tx, id, title, &current.document)?;
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
    fn invalid_transition_does_not_change_record() {
        let mut s = Store::open_memory().expect("store");
        let r = s
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("create");
        assert!(s.set_status(&r.id, Status::Superseded).is_err());
        assert_eq!(s.get(&r.id).expect("get").status, Status::Proposed);
    }

    #[test]
    fn retitle_preserves_document_history_and_reindexes_title() {
        let mut store = Store::open_memory().expect("store");
        let document = RecordKind::Adr.default_document("document content");
        let record = store
            .create(RecordKind::Adr, "Old unique title", document.clone())
            .expect("create");

        let updated = store
            .retitle(&record.id, "Renamed unique title")
            .expect("retitle");

        assert_eq!(updated.title, "Renamed unique title");
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.document, document);
        let history = store.history(&record.id).expect("history");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].document, history[1].document);
        assert_eq!(
            history[1].change_summary.as_deref(),
            Some("title changed to 'Renamed unique title'")
        );
        assert_eq!(
            store
                .search("Renamed", 10, 0)
                .expect("new title search")
                .len(),
            1
        );
        assert!(store
            .search("Old", 10, 0)
            .expect("old title search")
            .is_empty());
    }

    #[test]
    fn retitle_rejects_empty_title() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(
                RecordKind::Adr,
                "Original title",
                RecordKind::Adr.default_document("Original title"),
            )
            .expect("create");

        assert!(matches!(
            store.retitle(&record.id, "  "),
            Err(StoreError::Validation(ValidationError::EmptyTitle))
        ));
    }
}
