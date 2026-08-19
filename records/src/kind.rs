use kernel::sql_enum;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Scalar,
    List,
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
    }
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

    pub fn purpose(self) -> &'static str {
        match self {
            Self::Rfc => "Should this problem or proposal be pursued?",
            Self::Pdr => "What does the proposed design look like and what evidence supports it?",
            Self::Adr => "What architecturally significant choice was made and why?",
            Self::Edr => "What implementation-level engineering choice was made?",
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

    pub fn required_fields(self) -> &'static [(&'static str, FieldKind)] {
        match self {
            Self::Rfc => &[
                ("motivation", FieldKind::Scalar),
                ("problem", FieldKind::Scalar),
                ("scope", FieldKind::Scalar),
                ("non_goals", FieldKind::Scalar),
                ("constraints", FieldKind::Scalar),
                ("proposal", FieldKind::Scalar),
                ("alternatives", FieldKind::Scalar),
                ("questions_for_review", FieldKind::Scalar),
                ("outcome", FieldKind::Scalar),
            ],
            Self::Pdr => &[
                ("problem", FieldKind::Scalar),
                ("requirements", FieldKind::Scalar),
                ("constraints", FieldKind::Scalar),
                ("proposed_design", FieldKind::Scalar),
                ("components", FieldKind::Scalar),
                ("interfaces", FieldKind::Scalar),
                ("data_model", FieldKind::Scalar),
                ("failure_modes", FieldKind::Scalar),
                ("alternatives", FieldKind::Scalar),
                ("evidence", FieldKind::Scalar),
                ("experiments", FieldKind::Scalar),
                ("risks", FieldKind::Scalar),
                ("open_questions", FieldKind::Scalar),
                ("resulting_decisions", FieldKind::Scalar),
            ],
            Self::Adr | Self::Edr => &[
                ("context", FieldKind::Scalar),
                ("decision", FieldKind::Scalar),
                ("alternatives", FieldKind::Scalar),
                ("consequences", FieldKind::Scalar),
                ("evidence", FieldKind::Scalar),
            ],
        }
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
            let v = match field_kind {
                FieldKind::List => JsonValue::Array(Vec::new()),
                FieldKind::Scalar => JsonValue::String(if *field == "decision" {
                    format!("We will {title}.")
                } else {
                    title.to_string()
                }),
            };
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
        ];
        for (status, string) in expected {
            assert_eq!(status.to_string(), string);
            assert_eq!(Status::from_str(string), Ok(status));
        }
    }
}
