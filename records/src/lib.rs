use rusqlite::types::{FromSql, FromSqlError, ToSql, ToSqlOutput, Value, ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordKind {
    Rfc,
    Pdr,
    Adr,
    Edr,
}

impl RecordKind {
    pub const ALL: [Self; 4] = [Self::Rfc, Self::Pdr, Self::Adr, Self::Edr];
    pub fn code(self) -> &'static str {
        match self {
            Self::Rfc => "RFC",
            Self::Pdr => "PDR",
            Self::Adr => "ADR",
            Self::Edr => "EDR",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            Self::Rfc => "rfc",
            Self::Pdr => "pdr",
            Self::Adr => "adr",
            Self::Edr => "edr",
        }
    }
    pub fn initial_status(self) -> Status {
        match self {
            Self::Rfc | Self::Pdr => Status::Draft,
            Self::Adr | Self::Edr => Status::Proposed,
        }
    }
    pub fn statuses(self) -> &'static [Status] {
        match self {
            Self::Rfc => &[
                Status::Draft,
                Status::Proposed,
                Status::UnderReview,
                Status::Accepted,
                Status::Withdrawn,
            ],
            Self::Pdr => &[
                Status::Draft,
                Status::Review,
                Status::Approved,
                Status::Superseded,
            ],
            Self::Adr => &[
                Status::Proposed,
                Status::Accepted,
                Status::Deprecated,
                Status::Superseded,
            ],
            Self::Edr => &[Status::Proposed, Status::Accepted, Status::Superseded],
        }
    }
    pub fn required_fields(self) -> &'static [&'static str] {
        match self {
            Self::Rfc => &[
                "motivation",
                "problem",
                "scope",
                "non_goals",
                "constraints",
                "proposal",
                "alternatives",
                "questions_for_review",
                "outcome",
            ],
            Self::Pdr => &[
                "problem",
                "requirements",
                "constraints",
                "proposed_design",
                "components",
                "interfaces",
                "data_model",
                "failure_modes",
                "alternatives",
                "evidence",
                "experiments",
                "risks",
                "open_questions",
                "resulting_decisions",
            ],
            Self::Adr | Self::Edr => &[
                "context",
                "decision",
                "alternatives",
                "consequences",
                "evidence",
            ],
        }
    }
    pub fn default_document(self, title: &str) -> JsonValue {
        let mut m = Map::new();
        m.insert(
            "schema".into(),
            JsonValue::String(format!("{}/v1", self.slug())),
        );
        for field in self.required_fields() {
            let v = if *field == "alternatives"
                || *field == "evidence"
                || *field == "requirements"
                || *field == "constraints"
                || *field == "non_goals"
                || *field == "questions_for_review"
                || *field == "components"
                || *field == "interfaces"
                || *field == "failure_modes"
                || *field == "experiments"
                || *field == "risks"
                || *field == "open_questions"
                || *field == "resulting_decisions"
            {
                JsonValue::Array(Vec::new())
            } else {
                JsonValue::String(if *field == "decision" {
                    format!("We will {title}.")
                } else {
                    title.to_string()
                })
            };
            m.insert((*field).into(), v);
        }
        JsonValue::Object(m)
    }
}

impl fmt::Display for RecordKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl FromStr for RecordKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "rfc" => Ok(Self::Rfc),
            "pdr" => Ok(Self::Pdr),
            "adr" => Ok(Self::Adr),
            "edr" => Ok(Self::Edr),
            _ => Err(format!("unknown record kind: {s}")),
        }
    }
}
impl ToSql for RecordKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.code().to_string())))
    }
}
impl FromSql for RecordKind {
    fn column_result(value: ValueRef<'_>) -> Result<Self, FromSqlError> {
        Self::from_str(value.as_str()?)
            .map_err(|e| FromSqlError::Other(Box::new(ParseValueError(e))))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Draft,
    Proposed,
    UnderReview,
    Accepted,
    Withdrawn,
    Review,
    Approved,
    Deprecated,
    Superseded,
}
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::UnderReview => "under-review",
            Self::Accepted => "accepted",
            Self::Withdrawn => "withdrawn",
            Self::Review => "review",
            Self::Approved => "approved",
            Self::Deprecated => "deprecated",
            Self::Superseded => "superseded",
        })
    }
}
impl FromStr for Status {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "proposed" => Ok(Self::Proposed),
            "under-review" => Ok(Self::UnderReview),
            "accepted" => Ok(Self::Accepted),
            "withdrawn" => Ok(Self::Withdrawn),
            "review" => Ok(Self::Review),
            "approved" => Ok(Self::Approved),
            "deprecated" => Ok(Self::Deprecated),
            "superseded" => Ok(Self::Superseded),
            _ => Err(format!("unknown status: {s}")),
        }
    }
}
impl ToSql for Status {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}
impl FromSql for Status {
    fn column_result(value: ValueRef<'_>) -> Result<Self, FromSqlError> {
        Self::from_str(value.as_str()?)
            .map_err(|e| FromSqlError::Other(Box::new(ParseValueError(e))))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordId {
    pub kind: RecordKind,
    pub number: u32,
}
impl RecordId {
    pub fn new(kind: RecordKind, number: u32) -> Self {
        Self { kind, number }
    }
}
impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:04}", self.kind, self.number)
    }
}
impl FromStr for RecordId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (kind, number) = s
            .split_once('-')
            .ok_or_else(|| format!("invalid record id: {s}"))?;
        let kind = RecordKind::from_str(kind)?;
        let number = number
            .parse()
            .map_err(|_| format!("invalid record number: {s}"))?;
        Ok(Self::new(kind, number))
    }
}
impl ToSql for RecordId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}
impl FromSql for RecordId {
    fn column_result(value: ValueRef<'_>) -> Result<Self, FromSqlError> {
        Self::from_str(value.as_str()?)
            .map_err(|e| FromSqlError::Other(Box::new(ParseValueError(e))))
    }
}

#[derive(Debug)]
struct ParseValueError(String);
impl fmt::Display for ParseValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for ParseValueError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: RecordId,
    pub title: String,
    pub status: Status,
    pub document: JsonValue,
    pub revision: u32,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Revision {
    pub record_id: RecordId,
    pub revision: u32,
    pub document: JsonValue,
    pub changed_at: String,
    pub changed_by: Option<String>,
    pub change_summary: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub source_id: RecordId,
    pub relation: String,
    pub target_id: RecordId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub record_id: RecordId,
    pub kind: String,
    pub title: String,
    pub uri: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<JsonValue>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeReference {
    pub record_id: RecordId,
    pub relation: String,
    pub path: String,
    pub symbol: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
}

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
