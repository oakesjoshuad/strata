use kernel::sql_enum;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::str::FromStr;

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
