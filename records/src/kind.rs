use kernel::sql_enum;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::str::FromStr;

mod schema;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Scalar,
    List,
    Table,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportLayout {
    PerRecord,
    Aggregate { file: &'static str },
}

sql_enum! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    case_insensitive: true;
    pub enum RecordKind {
        Rfc => "RFC" | "rfc",
        Pdr => "PDR" | "pdr",
        Adr => "ADR" | "adr",
        Edr => "EDR" | "edr",
        Specification => "SPEC" | "spec",
        Assessment => "ASMT" | "asmt",
        Research => "RSCH" | "rsch",
        Glossary => "GLOS" | "glos",
        Risk => "RISK" | "risk",
    }
}

impl RecordKind {
    pub const ALL: [Self; 9] = [
        Self::Rfc,
        Self::Pdr,
        Self::Adr,
        Self::Edr,
        Self::Specification,
        Self::Assessment,
        Self::Research,
        Self::Glossary,
        Self::Risk,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Rfc => "RFC",
            Self::Pdr => "PDR",
            Self::Adr => "ADR",
            Self::Edr => "EDR",
            Self::Specification => "SPEC",
            Self::Assessment => "ASMT",
            Self::Research => "RSCH",
            Self::Glossary => "GLOS",
            Self::Risk => "RISK",
        }
    }

    pub fn purpose(self) -> &'static str {
        match self {
            Self::Rfc => "Should this problem or proposal be pursued?",
            Self::Pdr => "What does the proposed design look like and what evidence supports it?",
            Self::Adr => "What architecturally significant choice was made and why?",
            Self::Edr => "What implementation-level engineering choice was made?",
            Self::Specification => {
                "What must the system do, what constraints apply, and how is conformance verified?"
            }
            Self::Assessment => "What do we currently observe about this codebase, product, or conformance?",
            Self::Research => {
                "What does external or comparative evidence say about one specific, not-yet-decided question?"
            }
            Self::Glossary => "What does this term mean, and where does that meaning apply?",
            Self::Risk => "What could go wrong, how likely and severe is it, and what is being done about it?",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Rfc => "rfc",
            Self::Pdr => "pdr",
            Self::Adr => "adr",
            Self::Edr => "edr",
            Self::Specification => "specification",
            Self::Assessment => "assessment",
            Self::Research => "research",
            Self::Glossary => "glossary",
            Self::Risk => "risk",
        }
    }

    pub fn initial_status(self) -> Status {
        match self {
            Self::Rfc
            | Self::Pdr
            | Self::Adr
            | Self::Edr
            | Self::Specification
            | Self::Assessment
            | Self::Research
            | Self::Glossary => Status::Draft,
            Self::Risk => Status::Open,
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
                Status::Draft,
                Status::Proposed,
                Status::Accepted,
                Status::Deprecated,
                Status::Superseded,
            ],
            Self::Edr => &[
                Status::Draft,
                Status::Proposed,
                Status::Accepted,
                Status::Superseded,
            ],
            Self::Specification => &[
                Status::Draft,
                Status::Review,
                Status::Approved,
                Status::Superseded,
            ],
            Self::Assessment | Self::Research | Self::Glossary => {
                &[Status::Draft, Status::Accepted, Status::Superseded]
            }
            Self::Risk => &[
                Status::Open,
                Status::Monitoring,
                Status::Mitigated,
                Status::Accepted,
                Status::Materialized,
                Status::Closed,
            ],
        }
    }

    pub fn required_fields(self) -> &'static [(&'static str, FieldKind)] {
        schema::required_fields(self)
    }

    pub fn export_layout(self) -> ExportLayout {
        schema::export_layout(self)
    }

    pub fn field_kind(self, field: &str) -> Option<FieldKind> {
        self.required_fields()
            .iter()
            .find(|(name, _)| *name == field)
            .map(|(_, kind)| *kind)
    }

    pub fn default_document(self, title: &str) -> JsonValue {
        let mut m = Map::new();
        m.insert(
            "schema".into(),
            JsonValue::String(format!("{}/v1", self.slug())),
        );
        for (field, field_kind) in self.required_fields() {
            let v = schema::default_value(field, *field_kind, title);
            m.insert((*field).into(), v);
        }
        JsonValue::Object(m)
    }
}

sql_enum! {
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    case_insensitive: false;
    pub enum Status {
        Draft => "draft",
        Proposed => "proposed",
        UnderReview => "under-review",
        Accepted => "accepted",
        Withdrawn => "withdrawn",
        Review => "review",
        Approved => "approved",
        Deprecated => "deprecated",
        Superseded => "superseded",
        Open => "open",
        Monitoring => "monitoring",
        Mitigated => "mitigated",
        Materialized => "materialized",
        Closed => "closed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_kind_wire_strings_are_golden() {
        let expected = [
            (RecordKind::Rfc, "RFC"),
            (RecordKind::Pdr, "PDR"),
            (RecordKind::Adr, "ADR"),
            (RecordKind::Edr, "EDR"),
            (RecordKind::Specification, "SPEC"),
            (RecordKind::Assessment, "ASMT"),
            (RecordKind::Research, "RSCH"),
            (RecordKind::Glossary, "GLOS"),
            (RecordKind::Risk, "RISK"),
        ];
        for (kind, string) in expected {
            assert_eq!(kind.to_string(), string);
            assert_eq!(RecordKind::from_str(string), Ok(kind));
            assert_eq!(RecordKind::from_str(&string.to_ascii_lowercase()), Ok(kind));
        }
        assert_eq!(RecordKind::from_str("rFc"), Ok(RecordKind::Rfc));
    }

    #[test]
    fn record_kind_purposes_are_non_empty() {
        for kind in RecordKind::ALL {
            assert!(!kind.purpose().is_empty(), "{kind} has an empty purpose");
        }
    }

    #[test]
    fn status_wire_strings_are_golden() {
        let expected = [
            (Status::Draft, "draft"),
            (Status::Proposed, "proposed"),
            (Status::UnderReview, "under-review"),
            (Status::Accepted, "accepted"),
            (Status::Withdrawn, "withdrawn"),
            (Status::Review, "review"),
            (Status::Approved, "approved"),
            (Status::Deprecated, "deprecated"),
            (Status::Superseded, "superseded"),
            (Status::Open, "open"),
            (Status::Monitoring, "monitoring"),
            (Status::Mitigated, "mitigated"),
            (Status::Materialized, "materialized"),
            (Status::Closed, "closed"),
        ];
        for (status, string) in expected {
            assert_eq!(status.to_string(), string);
            assert_eq!(Status::from_str(string), Ok(status));
        }
    }
}
