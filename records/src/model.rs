use crate::{RecordId, Status};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: RecordId,
    pub title: String,
    pub slug: String,
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
