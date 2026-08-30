use crate::{Store, StoreError};
use records::{validate_document, validate_relationship, RecordId, RecordKind, RELATIONSHIPS};
use serde_json::Value as JsonValue;

mod specification;

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
        let mut rel = self
            .conn
            .prepare("SELECT source_id, relation, target_id FROM record_relation ORDER BY source_id, relation, target_id")?;
        for row in rel.query_map(rusqlite::named_params! {}, |r| {
            Ok((
                r.get::<_, RecordId>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, RecordId>(2)?,
            ))
        })? {
            let (source, relation, target) = row?;
            if validate_relationship(&source, &relation, &target).is_err()
                || !RELATIONSHIPS.contains(&relation.as_str())
            {
                issues.push(format!(
                    "broken or invalid relationship: {source} {relation} {target}"
                ));
            }
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
        let mut multiple_parents = self.conn.prepare(
            "SELECT target_id FROM record_relation WHERE relation = 'contains' GROUP BY target_id HAVING COUNT(*) > 1 ORDER BY target_id",
        )?;
        for row in
            multiple_parents.query_map(rusqlite::named_params! {}, |row| row.get::<_, String>(0))?
        {
            issues.push(format!("{} has more than one Specification parent", row?));
        }
        let mut cycles = self.conn.prepare(
            "WITH RECURSIVE reach(root, node) AS (SELECT source_id, target_id FROM record_relation WHERE relation = 'contains' UNION SELECT reach.root, relation.target_id FROM reach JOIN record_relation relation ON relation.source_id = reach.node AND relation.relation = 'contains') SELECT DISTINCT root FROM reach WHERE root = node ORDER BY root",
        )?;
        for row in cycles.query_map(rusqlite::named_params! {}, |row| row.get::<_, String>(0))? {
            issues.push(format!(
                "Specification hierarchy contains a cycle at {}",
                row?
            ));
        }
        issues.extend(specification::duplicate_requirement_issues(self)?);
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

    #[test]
    fn specification_hierarchy_reports_multiple_parents_and_cycles() {
        let mut store = Store::open_memory().expect("store");
        let first = store
            .create(
                RecordKind::Specification,
                "first",
                RecordKind::Specification.default_document("first"),
            )
            .expect("first");
        let second = store
            .create(
                RecordKind::Specification,
                "second",
                RecordKind::Specification.default_document("second"),
            )
            .expect("second");
        let third = store
            .create(
                RecordKind::Specification,
                "third",
                RecordKind::Specification.default_document("third"),
            )
            .expect("third");
        store.link(&first.id, "contains", &third.id).expect("link");
        store.link(&second.id, "contains", &third.id).expect("link");
        store.link(&third.id, "contains", &first.id).expect("link");

        let issues = store.validate().expect("validate");
        assert!(issues.iter().any(|issue| issue.contains("more than one")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("contains a cycle")));
    }
}
