mod id;
mod kind;
mod model;
mod validation;

pub use id::RecordId;
pub use kind::{RecordKind, Status};
pub use model::{CodeReference, Evidence, Record, Relationship, Revision};
pub use validation::{
    validate_document, validate_relationship, validate_transition, ValidationError, RELATIONSHIPS,
};
