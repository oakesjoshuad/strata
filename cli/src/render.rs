use crate::document::{fields, Block, Document, Frontmatter};
use crate::CliError;
use chrono::Utc;
use records::{Record, RecordId, RecordKind};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use store::Store;

pub(crate) fn render_record(store: &Store, id: &RecordId) -> Result<String, CliError> {
    let record = store.get(id)?;
    let relationships = store.outgoing_relationships(id)?;
    let document = document_from_record(&record, relationships)?;
    Ok(render_document(&document))
}

pub(crate) fn render_template(kind: RecordKind) -> Result<String, CliError> {
    let title = format!("Untitled {}", kind.code());
    let id = RecordId::new(kind, 0);
    let mut document = kind.default_document("");
    if let JsonValue::Object(values) = &mut document {
        for (field, _) in kind.required_fields() {
            if matches!(values.get(*field), Some(JsonValue::String(_))) {
                values.insert(
                    (*field).into(),
                    JsonValue::String(format!("TODO: {}", field.replace('_', " "))),
                );
            }
        }
    }
    let mut blocks = vec![Block::Heading {
        level: 1,
        text: format!("{id}: {title}"),
    }];
    blocks.extend(document_body(kind, &document)?);
    let frontmatter = Frontmatter {
        id,
        title,
        record_type: kind.slug(),
        status: kind.initial_status().to_string(),
        revision: 1,
        date: Utc::now().date_naive().to_string(),
        slug: format!("untitled-{}", kind.slug()),
        tags: Vec::new(),
        relationships: BTreeMap::new(),
    };
    Ok(render_document(&Document {
        frontmatter,
        blocks,
    }))
}

fn document_from_record(
    record: &Record,
    relationships: Vec<records::Relationship>,
) -> Result<Document, CliError> {
    let mut outgoing = BTreeMap::new();
    for relationship in relationships {
        outgoing
            .entry(relationship.relation)
            .or_insert_with(Vec::new)
            .push(relationship.target_id);
    }
    let tags = match record.document.get("tags") {
        Some(JsonValue::Array(values)) => values
            .iter()
            .map(|value| match value {
                JsonValue::String(value) => Ok(value.clone()),
                value => serde_json::to_string(value).map_err(CliError::Json),
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(CliError::Message("record tags must be a JSON array".into())),
        None => Vec::new(),
    };
    let mut blocks = vec![Block::Heading {
        level: 1,
        text: format!("{}: {}", record.id, record.title),
    }];
    blocks.extend(document_body(record.id.kind, &record.document)?);
    let date = record.created_at.chars().take(10).collect();
    Ok(Document {
        frontmatter: Frontmatter {
            id: record.id.clone(),
            title: record.title.clone(),
            record_type: record.id.kind.slug(),
            status: record.status.to_string(),
            revision: record.revision,
            date,
            slug: record.slug.clone(),
            tags,
            relationships: outgoing,
        },
        blocks,
    })
}

fn document_body(kind: RecordKind, document: &JsonValue) -> Result<Vec<Block>, CliError> {
    let mut blocks = Vec::new();
    for (field, heading) in fields(kind) {
        let value = document.get(*field).ok_or_else(|| {
            CliError::Message(format!("record document is missing field '{field}'"))
        })?;
        blocks.push(Block::Heading {
            level: 2,
            text: (*heading).into(),
        });
        blocks.extend(section_blocks(value)?);
    }
    Ok(blocks)
}

fn section_blocks(value: &JsonValue) -> Result<Vec<Block>, CliError> {
    match value {
        JsonValue::String(value) => Ok(vec![Block::Paragraph(value.clone())]),
        JsonValue::Array(values) => match string_values(values) {
            Some(items) => Ok(vec![Block::List(items)]),
            None => Ok(vec![Block::Paragraph(serde_json::to_string(value)?)]),
        },
        JsonValue::Object(value) => {
            if let Some(content) = value.get("code").and_then(JsonValue::as_str) {
                return Ok(vec![Block::Code {
                    language: value
                        .get("language")
                        .and_then(JsonValue::as_str)
                        .map(str::to_owned),
                    content: content.to_owned(),
                }]);
            }
            if let Some(content) = value.get("diagram").and_then(JsonValue::as_str) {
                return Ok(vec![Block::Diagram(content.to_owned())]);
            }
            if let Some(id) = value.get("record_reference").and_then(JsonValue::as_str) {
                return Ok(vec![Block::RecordReference {
                    id: id.parse().map_err(|error| {
                        CliError::Message(format!("invalid record reference: {error}"))
                    })?,
                    label: value
                        .get("label")
                        .and_then(JsonValue::as_str)
                        .map(str::to_owned),
                }]);
            }
            if let Some(id) = value.get("evidence_reference").and_then(JsonValue::as_str) {
                return Ok(vec![Block::EvidenceReference {
                    id: id.to_owned(),
                    label: value
                        .get("label")
                        .and_then(JsonValue::as_str)
                        .map(str::to_owned),
                }]);
            }
            if let (Some(headers), Some(rows)) = (value.get("headers"), value.get("rows")) {
                if let (Some(headers), Some(rows)) = (
                    headers.as_array().and_then(|values| string_values(values)),
                    rows.as_array().and_then(|rows| {
                        rows.iter()
                            .map(|row| row.as_array().and_then(|values| string_values(values)))
                            .collect::<Option<Vec<_>>>()
                    }),
                ) {
                    return Ok(vec![Block::Table { headers, rows }]);
                }
            }
            Ok(vec![Block::Paragraph(serde_json::to_string(value)?)])
        }
        value => Ok(vec![Block::Paragraph(serde_json::to_string(value)?)]),
    }
}

fn string_values(values: &[JsonValue]) -> Option<Vec<String>> {
    values
        .iter()
        .map(|value| value.as_str().map(str::to_owned))
        .collect()
}

fn render_document(document: &Document) -> String {
    let mut output = String::from("---\n");
    output.push_str(&format!(
        "id: {}\n",
        yaml_string(&document.frontmatter.id.to_string())
    ));
    output.push_str(&format!(
        "title: {}\n",
        yaml_string(&document.frontmatter.title)
    ));
    output.push_str(&format!(
        "record-type: {}\n",
        document.frontmatter.record_type
    ));
    output.push_str(&format!("status: {}\n", document.frontmatter.status));
    output.push_str(&format!("revision: {}\n", document.frontmatter.revision));
    output.push_str(&format!("date: {}\n", document.frontmatter.date));
    output.push_str(&format!("slug: {}\n", document.frontmatter.slug));
    if document.frontmatter.tags.is_empty() {
        output.push_str("tags: []\n");
    } else {
        output.push_str("tags:\n");
        for tag in &document.frontmatter.tags {
            output.push_str(&format!("  - {}\n", yaml_string(tag)));
        }
    }
    if document.frontmatter.relationships.is_empty() {
        output.push_str("relationships: {}\n");
    } else {
        output.push_str("relationships:\n");
        for (relation, targets) in &document.frontmatter.relationships {
            output.push_str(&format!("  {relation}:\n"));
            for target in targets {
                output.push_str(&format!("    - {}\n", yaml_string(&target.to_string())));
            }
        }
    }
    output.push_str("---\n\n");
    for block in &document.blocks {
        match block {
            Block::Heading { level, text } => {
                output.push_str(&format!("{} {}\n\n", "#".repeat(usize::from(*level)), text));
            }
            Block::Paragraph(content) => {
                output.push_str(&escape_paragraph(content.trim_end_matches('\n')));
                output.push_str("\n\n");
            }
            Block::List(items) => {
                for item in items {
                    output.push_str(&format!("- {item}\n"));
                }
                output.push('\n');
            }
            Block::Code { language, content } => {
                output.push_str(&format!(
                    "```{}\n{content}\n```\n\n",
                    language.as_deref().unwrap_or("")
                ));
            }
            Block::Table { headers, rows } => {
                output.push_str(&format!("| {} |\n", headers.join(" | ")));
                output.push_str(&format!(
                    "| {} |\n",
                    headers
                        .iter()
                        .map(|_| "---")
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
                for row in rows {
                    output.push_str(&format!("| {} |\n", row.join(" | ")));
                }
                output.push('\n');
            }
            Block::Diagram(content) => output.push_str(&format!("{content}\n\n")),
            Block::RecordReference { id, label } => {
                output.push_str(&format!(
                    "[{}]({})\n\n",
                    label.as_deref().unwrap_or("record"),
                    id
                ));
            }
            Block::EvidenceReference { id, label } => {
                output.push_str(&format!(
                    "[{}]({})\n\n",
                    label.as_deref().unwrap_or("evidence"),
                    id
                ));
            }
        }
    }
    while output.ends_with('\n') {
        output.pop();
    }
    output.push('\n');
    output
}

fn escape_paragraph(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            if line.starts_with("- ") {
                format!("\\{line}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn yaml_string(value: &str) -> String {
    match serde_json::to_string(value) {
        Ok(value) => value,
        Err(_) => "\"\"".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::RecordKind;

    #[test]
    fn rendering_is_byte_deterministic() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(
                RecordKind::Adr,
                "Deterministic rendering",
                RecordKind::Adr.default_document("Deterministic rendering"),
            )
            .expect("create");
        let first = render_record(&store, &record.id).expect("first render");
        let second = render_record(&store, &record.id).expect("second render");
        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn every_ir_block_family_has_a_renderer() {
        let record_id = RecordId::new(RecordKind::Adr, 1);
        let blocks = vec![
            Block::Heading {
                level: 1,
                text: "Heading".into(),
            },
            Block::Paragraph("Paragraph".into()),
            Block::List(vec!["item".into()]),
            Block::Code {
                language: Some("rust".into()),
                content: "fn main() {}".into(),
            },
            Block::Table {
                headers: vec!["A".into()],
                rows: vec![vec!["B".into()]],
            },
            Block::Diagram("diagram".into()),
            Block::RecordReference {
                id: record_id.clone(),
                label: Some("record".into()),
            },
            Block::EvidenceReference {
                id: "evidence-1".into(),
                label: Some("evidence".into()),
            },
        ];
        let document = Document {
            frontmatter: Frontmatter {
                id: record_id,
                title: "Test".into(),
                record_type: "adr",
                status: "draft".into(),
                revision: 1,
                date: "2026-01-01".into(),
                slug: "test".into(),
                tags: Vec::new(),
                relationships: BTreeMap::new(),
            },
            blocks,
        };
        let rendered = render_document(&document);
        assert!(rendered.contains("# Heading"));
        assert!(rendered.contains("- item"));
        assert!(rendered.contains("```rust"));
        assert!(rendered.contains("| A |"));
        assert!(rendered.contains("diagram"));
        assert!(rendered.contains("[record](ADR-0001)"));
        assert!(rendered.contains("[evidence](evidence-1)"));
    }
}
