use crate::{FieldKind, RecordId, RecordKind, Status};
use serde_json::Value as JsonValue;
use thiserror::Error;

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
    #[error("invalid status transition for {kind}: {from} -> {to}")]
    InvalidTransition {
        kind: RecordKind,
        from: Status,
        to: Status,
    },
    #[error("unsupported relationship: {0}")]
    InvalidRelationship(String),
    #[error("record ids must be different")]
    SelfRelationship,
    #[error("unsupported evidence kind: {0}")]
    InvalidEvidenceKind(String),
    #[error("evidence title must not be empty")]
    EmptyEvidenceTitle,
    #[error("code reference path must not be empty")]
    EmptyCodeReferencePath,
}

pub const RELATIONSHIPS: [&str; 10] = [
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
            (expected, _) => {
                return Err(ValidationError::WrongFieldType {
                    kind,
                    field: (*field).into(),
                    expected: *expected,
                });
            }
        }
    }
    Ok(())
}

pub fn validate_transition(
    kind: RecordKind,
    from: Status,
    to: Status,
) -> Result<(), ValidationError> {
    let valid = match kind {
        RecordKind::Rfc => matches!(
            (from, to),
            (Status::Draft, Status::Proposed)
                | (Status::Proposed, Status::UnderReview)
                | (Status::UnderReview, Status::Accepted)
                | (Status::UnderReview, Status::Withdrawn)
        ),
        RecordKind::Pdr => matches!(
            (from, to),
            (Status::Draft, Status::Review)
                | (Status::Review, Status::Approved)
                | (Status::Approved, Status::Superseded)
        ),
        RecordKind::Adr => matches!(
            (from, to),
            (Status::Draft, Status::Proposed)
                | (Status::Proposed, Status::Accepted)
                | (Status::Accepted, Status::Deprecated)
                | (Status::Accepted, Status::Superseded)
                | (Status::Deprecated, Status::Superseded)
        ),
        RecordKind::Edr => matches!(
            (from, to),
            (Status::Draft, Status::Proposed)
                | (Status::Proposed, Status::Accepted)
                | (Status::Accepted, Status::Superseded)
        ),
    };
    if valid {
        Ok(())
    } else {
        Err(ValidationError::InvalidTransition { kind, from, to })
    }
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
}
