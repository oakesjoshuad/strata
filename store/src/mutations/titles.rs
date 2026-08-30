use super::{index_record, insert_revision, timestamp, Store, StoreError};
use records::{Record, RecordId, ValidationError};

impl Store {
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
            rusqlite::named_params! { ":title": title, ":revision": revision, ":updated_at": &now, ":id": id },
        )?;
        let summary = format!("title changed to '{title}'");
        insert_revision(&tx, id, revision, &current.document, &summary)?;
        index_record(&tx, id, title, &current.document)?;
        tx.commit()?;
        self.get(id)
    }
}
