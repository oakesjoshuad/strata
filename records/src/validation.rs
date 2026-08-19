use crate::{RecordId, RecordKind, Status};
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

pub fn validate_document(kind: RecordKind, document: &JsonValue) -> Result<(), ValidationError> {
    let obj = document
        .as_object()
        .ok_or(ValidationError::DocumentNotObject)?;
    for field in kind.required_fields() {
        let Some(v) = obj.get(*field) else {
            return Err(ValidationError::MissingField {
                kind,
                field: (*field).into(),
            });
        };
        let empty = match v {
            JsonValue::String(s) => s.trim().is_empty(),
            JsonValue::Null => true,
            _ => false,
        };
        if empty {
            return Err(ValidationError::EmptyField {
                kind,
                field: (*field).into(),
            });
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
            (Status::Proposed, Status::Accepted)
                | (Status::Accepted, Status::Deprecated)
                | (Status::Accepted, Status::Superseded)
                | (Status::Deprecated, Status::Superseded)
        ),
        RecordKind::Edr => matches!(
            (from, to),
            (Status::Proposed, Status::Accepted) | (Status::Accepted, Status::Superseded)
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
    if !RELATIONSHIPS.contains(&relation) {
        return Err(ValidationError::InvalidRelationship(relation.into()));
    }
    if source == target {
        return Err(ValidationError::SelfRelationship);
    }
    Ok(())
}
