use super::{Store, StoreError};
use records::{validate_relationship, RecordId, Relationship};

impl Store {
    pub fn link(
        &mut self,
        source: &RecordId,
        relation: &str,
        target: &RecordId,
    ) -> Result<Relationship, StoreError> {
        self.link_many(source, relation, std::slice::from_ref(target))
            .map(|mut links| links.remove(0))
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
        let mut links = Vec::with_capacity(targets.len());
        for target in targets {
            tx.execute("INSERT INTO record_relation (source_id, relation, target_id) VALUES (:source, :relation, :target)", rusqlite::named_params! { ":source": source, ":relation": relation, ":target": target })?;
            links.push(Relationship {
                source_id: source.clone(),
                relation: relation.into(),
                target_id: target.clone(),
            });
        }
        tx.commit()?;
        Ok(links)
    }
}
