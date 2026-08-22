use crate::{
    index_record, insert_revision, insert_status_transition, slug, timestamp, Store, StoreError,
};
use records::{
    validate_document, validate_relationship, validate_transition, Record, RecordId, RecordKind,
    Relationship, Status, ValidationError,
};
use rusqlite::OptionalExtension;
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

    pub fn revise_patch(
        &mut self,
        id: &RecordId,
        patch: JsonValue,
        summary: Option<&str>,
    ) -> Result<Record, StoreError> {
        let patch = patch
            .as_object()
            .ok_or_else(|| StoreError::Argument("revise --patch requires a JSON object".into()))?;
        if patch.is_empty() {
            return Err(StoreError::Argument(
                "revise --patch requires at least one field".into(),
            ));
        }
        let current = self.get(id)?;
        let mut document = current.document.clone();
        let object = document.as_object_mut().ok_or_else(|| {
            StoreError::Argument("current record document is not a JSON object".into())
        })?;
        for (field, value) in patch {
            object.insert(field.clone(), value.clone());
        }
        self.revise(id, document, summary)
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
        insert_status_transition(&tx, id, revision, current.status, status, "forward")?;
        tx.commit()?;
        self.get(id)
    }

    pub fn undo_status(&mut self, id: &RecordId) -> Result<Record, StoreError> {
        let current = self.get(id)?;
        let transition = self
            .conn
            .query_row(
                "SELECT revision, from_status, to_status FROM status_transition WHERE record_id = :id AND revision = :revision AND transition_kind = :kind",
                rusqlite::named_params! {
                    ":id": id,
                    ":revision": current.revision,
                    ":kind": "forward",
                },
                |row| {
                    Ok((
                        row.get::<_, u32>(0)?,
                        row.get::<_, Status>(1)?,
                        row.get::<_, Status>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((transition_revision, from_status, to_status)) = transition else {
            return Err(StoreError::Argument(
                "status undo is only available immediately after a forward status transition"
                    .into(),
            ));
        };
        if transition_revision != current.revision || current.status != to_status {
            return Err(StoreError::Argument(
                "status undo audit does not match the current record status".into(),
            ));
        }
        let tx = self.conn.transaction()?;
        let revision = current.revision + 1;
        let now = timestamp();
        tx.execute(
            "UPDATE engineering_record SET status = :status, revision = :revision, updated_at = :updated_at WHERE id = :id",
            rusqlite::named_params! {
                ":status": from_status,
                ":revision": revision,
                ":updated_at": &now,
                ":id": id,
            },
        )?;
        let summary = format!("status undo: {to_status} -> {from_status}");
        insert_revision(&tx, id, revision, &current.document, &summary)?;
        insert_status_transition(&tx, id, revision, to_status, from_status, "undo")?;
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
        self.link_many(source, relation, std::slice::from_ref(target))
            .map(|mut v| v.remove(0))
    }

    pub fn link_many(
        &mut self,
        source: &RecordId,
        relation: &str,
        targets: &[RecordId],
    ) -> Result<Vec<Relationship>, StoreError> {
        if self.get(source).is_err() {
            return Err(StoreError::NotFound(source.clone()));
        }
        for target in targets {
            validate_relationship(source, relation, target)?;
            if self.get(target).is_err() {
                return Err(StoreError::NotFound(target.clone()));
            }
        }
        let tx = self.conn.transaction()?;
        let mut relationships = Vec::with_capacity(targets.len());
        for target in targets {
            tx.execute("INSERT INTO record_relation (source_id, relation, target_id) VALUES (:source, :relation, :target)", rusqlite::named_params! { ":source": source, ":relation": relation, ":target": target })?;
            relationships.push(Relationship {
                source_id: source.clone(),
                relation: relation.into(),
                target_id: target.clone(),
            });
        }
        tx.commit()?;
        Ok(relationships)
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
        assert_eq!(s.get(&r.id).expect("get").status, Status::Draft);
    }

    #[test]
    fn undo_status_reverses_only_the_latest_forward_transition() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(
                RecordKind::Adr,
                "Undo status",
                RecordKind::Adr.default_document("Undo status"),
            )
            .expect("create");

        let proposed = store
            .set_status(&record.id, Status::Proposed)
            .expect("propose");
        let undone = store.undo_status(&record.id).expect("undo");

        assert_eq!(proposed.status, Status::Proposed);
        assert_eq!(undone.status, Status::Draft);
        assert_eq!(undone.revision, 3);
        assert_eq!(
            store.history(&record.id).expect("history")[2]
                .change_summary
                .as_deref(),
            Some("status undo: proposed -> draft")
        );
        assert!(store.undo_status(&record.id).is_err());
    }

    #[test]
    fn undo_status_rejects_a_later_revision() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(
                RecordKind::Adr,
                "Undo status",
                RecordKind::Adr.default_document("Undo status"),
            )
            .expect("create");
        store
            .set_status(&record.id, Status::Proposed)
            .expect("propose");
        store
            .retitle(&record.id, "Later revision")
            .expect("retitle");

        assert!(store.undo_status(&record.id).is_err());
        assert_eq!(store.get(&record.id).expect("get").status, Status::Proposed);
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
    fn revise_patch_merges_fields_and_preserves_the_rest() {
        let mut store = Store::open_memory().expect("store");
        let original = RecordKind::Adr.default_document("original");
        let record = store
            .create(RecordKind::Adr, "Original title", original.clone())
            .expect("create");

        let updated = store
            .revise_patch(
                &record.id,
                serde_json::json!({"decision": "patched"}),
                Some("patched decision"),
            )
            .expect("patch");

        assert_eq!(updated.revision, 2);
        assert_eq!(updated.document["decision"], "patched");
        assert_eq!(updated.document["context"], original["context"]);
        assert_eq!(
            store.history(&record.id).expect("history")[1]
                .change_summary
                .as_deref(),
            Some("patched decision")
        );
    }

    #[test]
    fn revise_patch_rejects_empty_and_non_object_patches() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(
                RecordKind::Adr,
                "Original title",
                RecordKind::Adr.default_document("original"),
            )
            .expect("create");

        assert!(store
            .revise_patch(&record.id, serde_json::json!({}), None)
            .is_err());
        assert!(store
            .revise_patch(&record.id, serde_json::json!("not an object"), None)
            .is_err());
    }

    #[test]
    fn revise_rolls_back_completely_when_a_later_statement_fails() {
        let mut store = Store::open_memory().expect("store");
        let original = RecordKind::Adr.default_document("original");
        let record = store
            .create(RecordKind::Adr, "Original title", original.clone())
            .expect("create");

        // Seed a conflicting revision-2 row so revise()'s insert_revision call
        // fails after its UPDATE to engineering_record has already run, without
        // needing any fault-injection seam.
        store
            .conn
            .execute(
                "INSERT INTO record_revision (record_id, revision, document, changed_at, change_summary) VALUES (:id, 2, '{}', 'seed', 'seed')",
                rusqlite::named_params! { ":id": &record.id },
            )
            .expect("seed conflicting revision row");

        let result = store.revise(
            &record.id,
            RecordKind::Adr.default_document("revised"),
            None,
        );

        assert!(result.is_err());
        let after = store.get(&record.id).expect("get");
        assert_eq!(after.revision, 1);
        assert_eq!(after.document, original);
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

    #[test]
    fn link_many_rolls_back_completely_when_a_later_statement_fails() {
        let mut store = Store::open_memory().expect("store");
        let source = store
            .create(
                RecordKind::Adr,
                "Source",
                RecordKind::Adr.default_document("Source"),
            )
            .expect("source");
        let targets = (0..3)
            .map(|number| {
                store
                    .create(
                        RecordKind::Adr,
                        &format!("Target {number}"),
                        RecordKind::Adr.default_document(&format!("Target {number}")),
                    )
                    .expect("target")
            })
            .collect::<Vec<_>>();
        let target_ids = [
            targets[0].id.clone(),
            targets[1].id.clone(),
            targets[2].id.clone(),
            targets[0].id.clone(),
        ];

        assert!(store
            .link_many(&source.id, "relates-to", &target_ids)
            .is_err());
        let count: u32 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM record_relation WHERE source_id = :source",
                rusqlite::named_params! { ":source": &source.id },
                |row| row.get(0),
            )
            .expect("count relationships");
        assert_eq!(count, 0);
    }
}
