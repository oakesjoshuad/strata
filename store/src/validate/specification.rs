use super::Store;
use crate::StoreError;
use records::{Record, RecordId, RecordKind};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn duplicate_requirement_issues(store: &Store) -> Result<Vec<String>, StoreError> {
    let records = store
        .all_records()?
        .into_iter()
        .filter(|record| record.id.kind == RecordKind::Specification)
        .collect::<Vec<_>>();
    let records_by_id = records
        .iter()
        .map(|record| (record.id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let mut children = BTreeMap::<RecordId, Vec<RecordId>>::new();
    let mut children_with_parent = BTreeSet::new();
    for record in &records {
        for relationship in store.outgoing_relationships(&record.id)? {
            if relationship.relation == "contains" {
                children
                    .entry(record.id.clone())
                    .or_default()
                    .push(relationship.target_id.clone());
                children_with_parent.insert(relationship.target_id);
            }
        }
    }
    let roots = records
        .iter()
        .filter(|record| !children_with_parent.contains(&record.id))
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    for root in roots {
        let mut seen_records = BTreeSet::new();
        let mut requirements = BTreeMap::<String, RecordId>::new();
        collect_requirement_issues(
            &root,
            &records_by_id,
            &children,
            &mut seen_records,
            &mut requirements,
            &mut issues,
        );
    }
    Ok(issues)
}

fn collect_requirement_issues(
    id: &RecordId,
    records: &BTreeMap<RecordId, &Record>,
    children: &BTreeMap<RecordId, Vec<RecordId>>,
    seen_records: &mut BTreeSet<RecordId>,
    requirements: &mut BTreeMap<String, RecordId>,
    issues: &mut Vec<String>,
) {
    if !seen_records.insert(id.clone()) {
        return;
    }
    let Some(record) = records.get(id) else {
        return;
    };
    for requirement in requirement_ids(&record.document) {
        if let Some(first) = requirements.insert(requirement.clone(), id.clone()) {
            issues.push(format!(
                "duplicate Specification requirement {requirement} in {first} and {id}"
            ));
        }
    }
    if let Some(child_ids) = children.get(id) {
        for child in child_ids {
            collect_requirement_issues(
                child,
                records,
                children,
                seen_records,
                requirements,
                issues,
            );
        }
    }
}

fn requirement_ids(document: &JsonValue) -> Vec<String> {
    document
        .get("requirements")
        .and_then(|table| table.get("rows"))
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_array)
        .filter_map(|row| row.first())
        .filter_map(JsonValue::as_str)
        .map(str::to_owned)
        .collect()
}
