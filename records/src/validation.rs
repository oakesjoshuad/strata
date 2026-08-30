use crate::{FieldKind, RecordId, RecordKind, Status};
use serde_json::Value as JsonValue;
use thiserror::Error;

mod lifecycle;
mod specification;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("title must not be empty")]
    EmptyTitle,
    #[error("document must be a JSON object")]
    DocumentNotObject,
    #[error("{kind} document is missing required field '{field}'")]
    MissingField { kind: RecordKind, field: String },
    #[error("{kind} document field '{field}' must not be empty")]
    EmptyField { kind: RecordKind, field: String },
    #[error("{kind} document field '{field}' has wrong type; expected {expected:?}")]
    WrongFieldType {
        kind: RecordKind,
        field: String,
        expected: FieldKind,
    },
    #[error("{kind} document table field '{field}' is invalid: {detail}")]
    InvalidTable {
        kind: RecordKind,
        field: String,
        detail: String,
    },
    #[error("risk category must be one of technical, product, security, business, compliance; found '{0}'")]
    InvalidRiskCategory(String),
    #[error(
        "invalid status transition for {kind}: {from} -> {to} (valid from {from}: {valid_next})"
    )]
    InvalidTransition {
        kind: RecordKind,
        from: Status,
        to: Status,
        valid_next: String,
    },
    #[error("unsupported relationship: {0}")]
    InvalidRelationship(String),
    #[error("record ids must be different")]
    SelfRelationship,
    #[error("relationship '{relation}' cannot link {source_kind} to {target_kind}")]
    InvalidRelationshipEndpoint {
        relation: String,
        source_kind: RecordKind,
        target_kind: RecordKind,
    },
    #[error("unsupported evidence kind: {0}")]
    InvalidEvidenceKind(String),
    #[error("evidence title must not be empty")]
    EmptyEvidenceTitle,
    #[error("code reference path must not be empty")]
    EmptyCodeReferencePath,
}

pub const RELATIONSHIPS: [&str; 15] = [
    "relates-to",
    "derived-from",
    "explored-by",
    "produces",
    "resolves",
    "constrains",
    "implements",
    "implemented-by",
    "supported-by",
    "supersedes",
    "contains",
    "refines",
    "uses-term",
    "at-risk-from",
    "mitigates",
];

pub const EVIDENCE_KINDS: [&str; 7] = [
    "benchmark",
    "experiment",
    "source",
    "code-reference",
    "issue",
    "measurement",
    "prototype",
];

pub fn validate_document(kind: RecordKind, document: &JsonValue) -> Result<(), ValidationError> {
    let obj = document
        .as_object()
        .ok_or(ValidationError::DocumentNotObject)?;
    for (field, field_kind) in kind.required_fields() {
        let Some(v) = obj.get(*field) else {
            return Err(ValidationError::MissingField {
                kind,
                field: (*field).into(),
            });
        };
        match (field_kind, v) {
            (FieldKind::Scalar, JsonValue::String(value)) if value.trim().is_empty() => {
                return Err(ValidationError::EmptyField {
                    kind,
                    field: (*field).into(),
                });
            }
            (FieldKind::Scalar, JsonValue::String(_)) | (FieldKind::List, JsonValue::Array(_)) => {}
            (FieldKind::Table, value) if table_is_valid(value) => {}
            (expected, _) => {
                return Err(ValidationError::WrongFieldType {
                    kind,
                    field: (*field).into(),
                    expected: *expected,
                });
            }
        }
    }
    if kind == RecordKind::Risk {
        let category = obj
            .get("category")
            .and_then(JsonValue::as_str)
            .expect("Risk category was validated as a non-empty string");
        if !["technical", "product", "security", "business", "compliance"].contains(&category) {
            return Err(ValidationError::InvalidRiskCategory(category.into()));
        }
    }
    if kind == RecordKind::Specification {
        specification::validate_document(document)?;
    }
    Ok(())
}

fn table_is_valid(value: &JsonValue) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 2 || !object.contains_key("headers") || !object.contains_key("rows") {
        return false;
    }
    let Some(headers) = object.get("headers").and_then(JsonValue::as_array) else {
        return false;
    };
    if headers.iter().any(|header| !header.is_string()) {
        return false;
    }
    let Some(rows) = object.get("rows").and_then(JsonValue::as_array) else {
        return false;
    };
    rows.iter().all(|row| {
        row.as_array()
            .is_some_and(|cells| cells.iter().all(JsonValue::is_string))
    })
}

pub fn validate_transition(
    kind: RecordKind,
    from: Status,
    to: Status,
) -> Result<(), ValidationError> {
    let valid_next = valid_next_statuses(kind, from);
    if valid_next.contains(&to) {
        Ok(())
    } else {
        let valid_next = valid_next
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let valid_next = if valid_next.is_empty() {
            "none".into()
        } else {
            valid_next
        };
        Err(ValidationError::InvalidTransition {
            kind,
            from,
            to,
            valid_next,
        })
    }
}

pub fn valid_next_statuses(kind: RecordKind, from: Status) -> Vec<Status> {
    lifecycle::valid_next_statuses(kind, from)
}

pub fn validate_relationship(
    source: &RecordId,
    relation: &str,
    target: &RecordId,
) -> Result<(), ValidationError> {
    validate_relation_kind(relation)?;
    if source == target {
        return Err(ValidationError::SelfRelationship);
    }
    let valid_endpoints = match relation {
        "contains" => {
            source.kind == RecordKind::Specification && target.kind == RecordKind::Specification
        }
        "refines" => target.kind == RecordKind::Specification,
        "uses-term" => target.kind == RecordKind::Glossary,
        "at-risk-from" | "mitigates" => target.kind == RecordKind::Risk,
        _ => true,
    };
    if !valid_endpoints {
        return Err(ValidationError::InvalidRelationshipEndpoint {
            relation: relation.into(),
            source_kind: source.kind,
            target_kind: target.kind,
        });
    }
    Ok(())
}

pub fn validate_relation_kind(relation: &str) -> Result<(), ValidationError> {
    if !RELATIONSHIPS.contains(&relation) {
        return Err(ValidationError::InvalidRelationship(relation.into()));
    }
    Ok(())
}

pub fn validate_evidence(kind: &str, title: &str) -> Result<(), ValidationError> {
    if !EVIDENCE_KINDS.contains(&kind) {
        return Err(ValidationError::InvalidEvidenceKind(kind.into()));
    }
    if title.trim().is_empty() {
        return Err(ValidationError::EmptyEvidenceTitle);
    }
    Ok(())
}

pub fn validate_code_reference(relation: &str, path: &str) -> Result<(), ValidationError> {
    validate_relation_kind(relation)?;
    if path.trim().is_empty() {
        return Err(ValidationError::EmptyCodeReferencePath);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scalar_field_array_is_rejected_with_wrong_type() {
        let mut document = RecordKind::Adr.default_document("title");
        document["context"] = json!([]);

        assert!(matches!(
            validate_document(RecordKind::Adr, &document),
            Err(ValidationError::WrongFieldType {
                kind: RecordKind::Adr,
                field,
                expected: FieldKind::Scalar,
            }) if field == "context"
        ));
    }

    #[test]
    fn all_scalar_default_fields_are_valid() {
        let document = RecordKind::Adr.default_document("title");

        assert!(validate_document(RecordKind::Adr, &document).is_ok());
    }

    #[test]
    fn adr_and_edr_draft_transitions_require_proposed_first() {
        for kind in [RecordKind::Adr, RecordKind::Edr] {
            assert!(validate_transition(kind, Status::Draft, Status::Proposed).is_ok());
            assert!(validate_transition(kind, Status::Proposed, Status::Accepted).is_ok());
            assert!(validate_transition(kind, Status::Draft, Status::Accepted).is_err());
        }
    }

    #[test]
    fn valid_next_statuses_follow_each_kind_lifecycle() {
        assert_eq!(
            valid_next_statuses(RecordKind::Rfc, Status::UnderReview),
            vec![Status::Accepted, Status::Withdrawn]
        );
        assert_eq!(
            valid_next_statuses(RecordKind::Pdr, Status::Approved),
            vec![Status::Superseded]
        );
        assert_eq!(
            valid_next_statuses(RecordKind::Adr, Status::Accepted),
            vec![Status::Deprecated, Status::Superseded]
        );
        assert_eq!(
            valid_next_statuses(RecordKind::Edr, Status::Superseded),
            Vec::<Status>::new()
        );
        assert_eq!(
            valid_next_statuses(RecordKind::Risk, Status::Open),
            vec![
                Status::Monitoring,
                Status::Mitigated,
                Status::Accepted,
                Status::Materialized,
            ]
        );
    }

    #[test]
    fn evidence_kind_outside_the_whitelist_is_rejected() {
        assert!(matches!(
            validate_evidence("vibes", "a title"),
            Err(ValidationError::InvalidEvidenceKind(kind)) if kind == "vibes"
        ));
    }

    #[test]
    fn evidence_title_must_not_be_empty() {
        assert!(matches!(
            validate_evidence("benchmark", "  "),
            Err(ValidationError::EmptyEvidenceTitle)
        ));
    }

    #[test]
    fn code_reference_relation_must_be_a_known_kind() {
        assert!(matches!(
            validate_code_reference("vibes", "src/lib.rs"),
            Err(ValidationError::InvalidRelationship(relation)) if relation == "vibes"
        ));
    }

    #[test]
    fn code_reference_path_must_not_be_empty() {
        assert!(matches!(
            validate_code_reference("constrains", "  "),
            Err(ValidationError::EmptyCodeReferencePath)
        ));
    }

    #[test]
    fn new_relationships_enforce_their_endpoint_kinds() {
        let specification = RecordId::new(RecordKind::Specification, 1);
        let glossary = RecordId::new(RecordKind::Glossary, 1);
        let risk = RecordId::new(RecordKind::Risk, 1);
        let adr = RecordId::new(RecordKind::Adr, 1);
        assert!(validate_relationship(
            &specification,
            "contains",
            &RecordId::new(RecordKind::Specification, 2)
        )
        .is_ok());
        assert!(validate_relationship(&adr, "refines", &specification).is_ok());
        assert!(validate_relationship(&adr, "uses-term", &glossary).is_ok());
        assert!(validate_relationship(&adr, "at-risk-from", &risk).is_ok());
        assert!(validate_relationship(&adr, "mitigates", &risk).is_ok());
        assert!(validate_relationship(&adr, "contains", &specification).is_err());
        assert!(validate_relationship(&adr, "uses-term", &risk).is_err());
    }
}
