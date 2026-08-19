mod id;
mod kind;
mod model;
mod validation;

pub use id::RecordId;
pub use kind::{FieldKind, RecordKind, Status};
pub use model::{CodeReference, Evidence, Record, Relationship, Revision};
pub use validation::{
    validate_code_reference, validate_document, validate_evidence, validate_relationship,
    validate_transition, ValidationError, EVIDENCE_KINDS, RELATIONSHIPS,
};
