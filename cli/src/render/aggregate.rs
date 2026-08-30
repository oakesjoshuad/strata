use crate::document::fields;
use crate::CliError;
use records::{Record, RecordKind, Status};
use serde_json::Value as JsonValue;

pub(crate) fn render_aggregate(kind: RecordKind, records: &[&Record]) -> Result<String, CliError> {
    match kind {
        RecordKind::Glossary => render_glossary(records),
        RecordKind::Risk => render_risk_register(records),
        _ => Err(CliError::Message(format!(
            "{} does not have an aggregate export layout",
            kind.code()
        ))),
    }
}

fn render_glossary(records: &[&Record]) -> Result<String, CliError> {
    let mut records = records.to_vec();
    records.sort_by(|left, right| {
        left.slug
            .cmp(&right.slug)
            .then(left.id.number.cmp(&right.id.number))
    });
    let mut output = aggregate_preamble("glossary", "Glossary");
    for record in records {
        append_entry(&mut output, record)?;
    }
    Ok(output)
}

fn render_risk_register(records: &[&Record]) -> Result<String, CliError> {
    let mut records = records.to_vec();
    records.sort_by(|left, right| {
        risk_status_rank(left.status)
            .cmp(&risk_status_rank(right.status))
            .then(left.id.number.cmp(&right.id.number))
    });
    let mut output = aggregate_preamble("risk", "Risk Register");
    output.push_str("| ID | Title | Status | Likelihood | Impact | Owner |\n");
    output.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for record in &records {
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            record.id,
            record.title,
            record.status,
            scalar(record, "likelihood")?,
            scalar(record, "impact")?,
            scalar(record, "owner")?,
        ));
    }
    output.push('\n');
    for record in records {
        append_entry(&mut output, record)?;
    }
    Ok(output)
}

fn aggregate_preamble(record_type: &str, title: &str) -> String {
    format!("---\nrecord-type: {record_type}\n---\n\n# {title}\n\n")
}

fn append_entry(output: &mut String, record: &Record) -> Result<(), CliError> {
    let anchor = entry_anchor(record);
    output.push_str(&format!(
        "## {}: {} {{#{anchor}}}\n\n",
        record.id, record.title
    ));
    for (field, heading) in fields(record.id.kind) {
        output.push_str(&format!(
            "### {heading} {{#{anchor}-{}}}\n\n",
            field.replace('_', "-")
        ));
        output.push_str(&render_value(record.document.get(*field).ok_or_else(
            || CliError::Message(format!("record document is missing field '{field}'")),
        )?)?);
        output.push_str("\n\n");
    }
    Ok(())
}

fn render_value(value: &JsonValue) -> Result<String, CliError> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Array(values) => values
            .iter()
            .map(|value| match value.as_str() {
                Some(value) => Ok(format!("- {value}")),
                None => Ok(serde_json::to_string(value)?),
            })
            .collect::<Result<Vec<_>, CliError>>()
            .map(|items| items.join("\n")),
        JsonValue::Object(value) if value.contains_key("headers") && value.contains_key("rows") => {
            let headers = value
                .get("headers")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| CliError::Message("table headers must be an array".into()))?;
            let rows = value
                .get("rows")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| CliError::Message("table rows must be an array".into()))?;
            let headers = strings(headers)?;
            let mut output = format!(
                "| {} |\n|{}|",
                headers.join(" | "),
                " --- |".repeat(headers.len())
            );
            for row in rows {
                let cells = row
                    .as_array()
                    .ok_or_else(|| CliError::Message("table row must be an array".into()))?;
                output.push_str(&format!("\n| {} |", strings(cells)?.join(" | ")));
            }
            Ok(output)
        }
        value => Ok(serde_json::to_string(value)?),
    }
}

fn scalar<'a>(record: &'a Record, field: &str) -> Result<&'a str, CliError> {
    record
        .document
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            CliError::Message(format!("{} is missing scalar field '{field}'", record.id))
        })
}

fn strings(values: &[JsonValue]) -> Result<Vec<&str>, CliError> {
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| CliError::Message("table values must be strings".into()))
        })
        .collect()
}

fn entry_anchor(record: &Record) -> String {
    format!("{:04}-{}", record.id.number, record.slug)
}

fn risk_status_rank(status: Status) -> u8 {
    match status {
        Status::Open => 0,
        Status::Monitoring => 1,
        Status::Materialized => 2,
        Status::Accepted => 3,
        Status::Mitigated => 4,
        Status::Closed => 5,
        Status::Draft
        | Status::Proposed
        | Status::UnderReview
        | Status::Withdrawn
        | Status::Review
        | Status::Approved
        | Status::Deprecated
        | Status::Superseded => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::RecordKind;

    #[test]
    fn glossary_anchors_are_derived_from_immutable_identity() {
        let record = Record {
            id: records::RecordId::new(RecordKind::Glossary, 3),
            title: "Lead".into(),
            slug: "lead".into(),
            status: Status::Draft,
            document: RecordKind::Glossary.default_document("Lead"),
            revision: 1,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let rendered = render_glossary(&[&record]).expect("render");
        assert!(rendered.contains("## GLOS-0003: Lead {#0003-lead}"));
        assert!(rendered.contains("### Definition {#0003-lead-definition}"));
    }
}
