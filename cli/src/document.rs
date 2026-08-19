use records::RecordId;
use std::collections::BTreeMap;

// The full block vocabulary is part of ADR-0008. This pass maps section
// boundaries and opaque content; later parser and publication work will use
// the remaining variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    List(Vec<String>),
    Code {
        language: Option<String>,
        content: String,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Diagram(String),
    RecordReference {
        id: RecordId,
        label: Option<String>,
    },
    EvidenceReference {
        id: String,
        label: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frontmatter {
    pub id: RecordId,
    pub title: String,
    pub record_type: &'static str,
    pub status: String,
    pub revision: u32,
    pub date: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub relationships: BTreeMap<String, Vec<RecordId>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Document {
    pub frontmatter: Frontmatter,
    pub blocks: Vec<Block>,
}

pub(crate) fn fields(kind: records::RecordKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        records::RecordKind::Rfc => &[
            ("motivation", "Motivation"),
            ("problem", "Problem"),
            ("scope", "Scope"),
            ("non_goals", "Non-Goals"),
            ("constraints", "Constraints"),
            ("proposal", "Proposal"),
            ("alternatives", "Alternatives Considered"),
            ("questions_for_review", "Open Questions"),
            ("outcome", "Outcome"),
        ],
        records::RecordKind::Pdr => &[
            ("problem", "Problem"),
            ("requirements", "Requirements"),
            ("constraints", "Constraints"),
            ("proposed_design", "Proposed Design"),
            ("components", "Components"),
            ("interfaces", "Interfaces"),
            ("data_model", "Data Model"),
            ("failure_modes", "Failure Modes"),
            ("alternatives", "Alternatives Considered"),
            ("evidence", "Evidence"),
            ("experiments", "Experiments"),
            ("risks", "Risks"),
            ("open_questions", "Open Questions"),
            ("resulting_decisions", "Resulting Decisions"),
        ],
        records::RecordKind::Adr | records::RecordKind::Edr => &[
            ("context", "Context"),
            ("decision", "Decision"),
            ("alternatives", "Considered Options"),
            ("consequences", "Consequences"),
            ("evidence", "Evidence"),
        ],
    }
}
