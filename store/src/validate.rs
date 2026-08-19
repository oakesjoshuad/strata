use crate::{Store, StoreError};
use records::{validate_document, RecordId, RecordKind};
use serde_json::Value as JsonValue;

impl Store {
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
        let mut lineage = self.conn.prepare(
            "SELECT t.id FROM engineering_record t WHERE t.status = 'accepted' AND t.kind IN ('ADR', 'EDR') AND NOT EXISTS (SELECT 1 FROM record_relation rr JOIN engineering_record s ON s.id = rr.source_id WHERE rr.target_id = t.id AND rr.relation IN ('produces', 'derived-from') AND s.kind IN ('RFC', 'PDR')) ORDER BY t.id",
        )?;
        for row in lineage.query_map([], |r| {
            Ok(format!(
                "WARN {}: accepted decision has no qualifying lineage from an RFC or PDR",
                r.get::<_, String>(0)?
            ))
        })? {
            issues.push(row?);
        }
        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::{RecordKind, Status};

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
            .set_status(&target.id, Status::Proposed)
            .expect("propose target");
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

    #[test]
    fn accepted_decision_without_lineage_is_reported_as_warning() {
        let mut store = Store::open_memory().expect("store");
        let decision = store
            .create(
                RecordKind::Adr,
                "standalone",
                RecordKind::Adr.default_document("standalone"),
            )
            .expect("create decision");
        store
            .set_status(&decision.id, Status::Proposed)
            .expect("propose decision");
        store
            .set_status(&decision.id, Status::Accepted)
            .expect("accept decision");

        let issues = store.validate().expect("validate");
        assert!(issues.iter().any(|issue| {
            issue == "WARN ADR-0001: accepted decision has no qualifying lineage from an RFC or PDR"
        }));
    }
}
