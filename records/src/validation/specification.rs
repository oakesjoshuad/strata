use super::ValidationError;
use crate::RecordKind;
use kernel::sql_enum;
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;
use std::str::FromStr;

sql_enum! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    case_insensitive: false;
    pub enum RequirementStatus {
        Planned => "planned",
        InProgress => "in-progress",
        Verified => "verified",
        Blocked => "blocked",
        Waived => "waived",
    }
}

pub(super) fn validate_document(document: &JsonValue) -> Result<(), ValidationError> {
    let requirements = table(document, "requirements")?;
    let criteria = table(document, "acceptance_criteria")?;
    check_headers(
        "requirements",
        requirements,
        &["id", "requirement", "status"],
    )?;
    check_headers(
        "acceptance_criteria",
        criteria,
        &["requirement", "criterion"],
    )?;

    let mut identifiers = BTreeSet::new();
    for row in rows("requirements", requirements, 3)? {
        let id = &row[0];
        if !valid_requirement_id(id) {
            return invalid("requirements", format!("invalid requirement id '{id}'"));
        }
        if !identifiers.insert(id.clone()) {
            return invalid("requirements", format!("duplicate requirement id '{id}'"));
        }
        if RequirementStatus::from_str(&row[2]).is_err() {
            return invalid(
                "requirements",
                format!("invalid requirement status '{}'", row[2]),
            );
        }
    }
    for row in rows("acceptance_criteria", criteria, 2)? {
        if !identifiers.contains(&row[0]) {
            return invalid(
                "acceptance_criteria",
                format!("criterion references unknown requirement '{}'", row[0]),
            );
        }
    }
    Ok(())
}

fn table<'a>(
    document: &'a JsonValue,
    field: &str,
) -> Result<&'a serde_json::Map<String, JsonValue>, ValidationError> {
    document
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| ValidationError::InvalidTable {
            kind: RecordKind::Specification,
            field: field.into(),
            detail: "must be an object".into(),
        })
}

fn check_headers(
    field: &str,
    table: &serde_json::Map<String, JsonValue>,
    expected: &[&str],
) -> Result<(), ValidationError> {
    let headers = table
        .get("headers")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| invalid_error(field, "headers must be an array"))?;
    let actual = headers
        .iter()
        .map(JsonValue::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| invalid_error(field, "headers must contain only strings"))?;
    if actual != expected {
        return invalid(field, format!("headers must be {}", expected.join(", ")));
    }
    Ok(())
}

fn rows(
    field: &str,
    table: &serde_json::Map<String, JsonValue>,
    width: usize,
) -> Result<Vec<Vec<String>>, ValidationError> {
    let rows = table
        .get("rows")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| invalid_error(field, "rows must be an array"))?;
    rows.iter()
        .map(|row| {
            let row = row
                .as_array()
                .ok_or_else(|| invalid_error(field, "every row must be an array"))?;
            let cells = row
                .iter()
                .map(JsonValue::as_str)
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| invalid_error(field, "every table cell must be a string"))?;
            if cells.len() != width {
                return Err(invalid_error(
                    field,
                    format!("every row must contain {width} cells"),
                ));
            }
            Ok(cells
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<String>>())
        })
        .collect()
}

fn valid_requirement_id(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    let Some((last, middle)) = parts.split_last() else {
        return false;
    };
    parts.len() >= 3
        && middle.first() == Some(&"REQ")
        && last.len() == 3
        && last.chars().all(|character| character.is_ascii_digit())
        && middle[1..].iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
        })
}

fn invalid(field: &str, detail: impl Into<String>) -> Result<(), ValidationError> {
    Err(invalid_error(field, detail))
}

fn invalid_error(field: &str, detail: impl Into<String>) -> ValidationError {
    ValidationError::InvalidTable {
        kind: RecordKind::Specification,
        field: field.into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_machine_readable_requirement_tables() {
        let document = json!({
            "requirements": {"headers": ["id", "requirement", "status"], "rows": [["REQ-CLI-001", "The CLI accepts a specification", "planned"]]},
            "acceptance_criteria": {"headers": ["requirement", "criterion"], "rows": [["REQ-CLI-001", "A test creates SPEC-0001"]]}
        });
        assert!(validate_document(&document).is_ok());
    }

    #[test]
    fn rejects_unknown_criterion_requirement() {
        let document = json!({
            "requirements": {"headers": ["id", "requirement", "status"], "rows": []},
            "acceptance_criteria": {"headers": ["requirement", "criterion"], "rows": [["REQ-CLI-001", "A test"]]}
        });
        assert!(validate_document(&document).is_err());
    }
}
