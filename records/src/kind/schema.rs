use super::{ExportLayout, FieldKind, RecordKind};
use serde_json::{json, Value as JsonValue};

pub(super) fn required_fields(kind: RecordKind) -> &'static [(&'static str, FieldKind)] {
    match kind {
        RecordKind::Rfc => &[
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
        RecordKind::Pdr => &[
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
        RecordKind::Adr | RecordKind::Edr => &[
            ("context", FieldKind::Scalar),
            ("decision", FieldKind::Scalar),
            ("alternatives", FieldKind::Scalar),
            ("consequences", FieldKind::Scalar),
            ("evidence", FieldKind::Scalar),
        ],
        RecordKind::Specification => &[
            ("purpose", FieldKind::Scalar),
            ("scope", FieldKind::Scalar),
            ("non_goals", FieldKind::Scalar),
            ("assumptions", FieldKind::Scalar),
            ("constraints", FieldKind::Scalar),
            ("requirements", FieldKind::Table),
            ("acceptance_criteria", FieldKind::Table),
            ("verification", FieldKind::Scalar),
            ("open_questions", FieldKind::Scalar),
        ],
        RecordKind::Assessment => &[
            ("subject", FieldKind::Scalar),
            ("method", FieldKind::Scalar),
            ("observations", FieldKind::Scalar),
            ("gaps", FieldKind::Scalar),
            ("confidence", FieldKind::Scalar),
            ("recommendations", FieldKind::Scalar),
        ],
        RecordKind::Research => &[
            ("question", FieldKind::Scalar),
            ("method", FieldKind::Scalar),
            ("findings", FieldKind::Scalar),
            ("comparison", FieldKind::Scalar),
            ("implications", FieldKind::Scalar),
            ("limitations", FieldKind::Scalar),
        ],
        RecordKind::Glossary => &[
            ("definition", FieldKind::Scalar),
            ("aliases", FieldKind::List),
            ("scope", FieldKind::Scalar),
            ("examples", FieldKind::Scalar),
            ("non_examples", FieldKind::Scalar),
        ],
        RecordKind::Risk => &[
            ("subject", FieldKind::Scalar),
            ("description", FieldKind::Scalar),
            ("category", FieldKind::Scalar),
            ("likelihood", FieldKind::Scalar),
            ("impact", FieldKind::Scalar),
            ("mitigation", FieldKind::Scalar),
            ("owner", FieldKind::Scalar),
        ],
    }
}

pub(super) fn export_layout(kind: RecordKind) -> ExportLayout {
    match kind {
        RecordKind::Glossary => ExportLayout::Aggregate {
            file: "glossary.md",
        },
        RecordKind::Risk => ExportLayout::Aggregate {
            file: "risk-register.md",
        },
        RecordKind::Rfc
        | RecordKind::Pdr
        | RecordKind::Adr
        | RecordKind::Edr
        | RecordKind::Specification
        | RecordKind::Assessment
        | RecordKind::Research => ExportLayout::PerRecord,
    }
}

pub(super) fn default_value(field: &str, kind: FieldKind, title: &str) -> JsonValue {
    match (field, kind) {
        ("requirements", FieldKind::Table) => {
            json!({"headers": ["id", "requirement", "status"], "rows": []})
        }
        ("acceptance_criteria", FieldKind::Table) => {
            json!({"headers": ["requirement", "criterion"], "rows": []})
        }
        ("category", FieldKind::Scalar) => JsonValue::String("technical".into()),
        (_, FieldKind::List) => JsonValue::Array(Vec::new()),
        ("decision", FieldKind::Scalar) => JsonValue::String(format!("We will {title}.")),
        (_, FieldKind::Scalar) => JsonValue::String(title.to_string()),
        (_, FieldKind::Table) => JsonValue::Object(Default::default()),
    }
}
