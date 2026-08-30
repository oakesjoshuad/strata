use crate::document::{fields, Block, Document, Frontmatter};
use crate::CliError;
use records::{RecordId, RecordKind};
use serde_json::{json, Map, Value as JsonValue};
use std::fs;
use std::io::Read;
use std::path::Path;

mod frontmatter;

pub(crate) fn read_file(path: &Path, kind: RecordKind) -> Result<JsonValue, CliError> {
    let source = path.display().to_string();
    let text = fs::read_to_string(path).map_err(|error| source_error(&source, kind, error))?;
    parse(&source, kind, &text)
}

pub(crate) fn read_stdin(kind: RecordKind) -> Result<JsonValue, CliError> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|error| source_error("<stdin>", kind, error))?;
    parse("<stdin>", kind, &text)
}

pub(crate) fn read_file_for_id(path: &Path, id: &RecordId) -> Result<JsonValue, CliError> {
    let source = path.display().to_string();
    let text = fs::read_to_string(path).map_err(|error| source_error(&source, id.kind, error))?;
    parse_for_id(&source, id, &text)
}

pub(crate) fn read_stdin_for_id(id: &RecordId) -> Result<JsonValue, CliError> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|error| source_error("<stdin>", id.kind, error))?;
    parse_for_id("<stdin>", id, &text)
}

fn parse(source: &str, kind: RecordKind, text: &str) -> Result<JsonValue, CliError> {
    parse_for_kind(source, kind, text, None)
}

fn parse_for_id(source: &str, id: &RecordId, text: &str) -> Result<JsonValue, CliError> {
    parse_for_kind(source, id.kind, text, Some(id))
}

fn parse_for_kind(
    source: &str,
    kind: RecordKind,
    text: &str,
    expected_id: Option<&RecordId>,
) -> Result<JsonValue, CliError> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().copied() != Some("---") {
        return Err(input_error(
            source,
            kind,
            "frontmatter must begin with '---' on the first line",
        ));
    }
    let closing = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (*line == "---").then_some(index))
        .ok_or_else(|| input_error(source, kind, "frontmatter is missing its closing '---'"))?;
    let yaml = lines[1..closing].join("\n");
    let frontmatter = frontmatter::parse(source, kind, &yaml)?;
    if let Some(expected_id) = expected_id {
        if frontmatter.id != *expected_id {
            return Err(input_error(
                source,
                kind,
                format!(
                    "frontmatter id is {}, expected {}",
                    frontmatter.id, expected_id
                ),
            ));
        }
    }
    let body = &lines[closing + 1..];
    let document = parse_body(source, kind, &frontmatter, body)?;
    document_to_json(source, kind, document)
}

fn parse_body(
    source: &str,
    kind: RecordKind,
    frontmatter: &Frontmatter,
    lines: &[&str],
) -> Result<Document, CliError> {
    let mut index = 0;
    while lines.get(index).is_some_and(|line| line.trim().is_empty()) {
        index += 1;
    }
    let title_heading = format!("# {}: {}", frontmatter.id, frontmatter.title);
    if lines.get(index).copied() != Some(title_heading.as_str()) {
        return Err(input_error(
            source,
            kind,
            "body is missing the exact level-one title heading",
        ));
    }
    index += 1;
    let mut blocks = vec![Block::Heading {
        level: 1,
        text: format!("{}: {}", frontmatter.id, frontmatter.title),
        id: None,
    }];
    for (field, heading) in fields(kind) {
        while lines.get(index).is_some_and(|line| line.trim().is_empty()) {
            index += 1;
        }
        let expected = format!("## {heading}");
        if lines.get(index).copied() != Some(expected.as_str()) {
            let detail = lines
                .get(index)
                .map(|line| {
                    format!("section '{field}' must be headed by '{expected}', found '{line}'")
                })
                .unwrap_or_else(|| format!("missing required section '{field}'"));
            return Err(input_error(source, kind, detail));
        }
        blocks.push(Block::Heading {
            level: 2,
            text: (*heading).into(),
            id: None,
        });
        index += 1;
        let start = index;
        while let Some(line) = lines.get(index) {
            if line.starts_with('#') {
                break;
            }
            index += 1;
        }
        blocks.extend(parse_section(source, kind, field, &lines[start..index])?);
    }
    while lines.get(index).is_some_and(|line| line.trim().is_empty()) {
        index += 1;
    }
    if let Some(line) = lines.get(index) {
        return Err(input_error(
            source,
            kind,
            format!("unexpected trailing heading or content '{line}'"),
        ));
    }
    Ok(Document {
        frontmatter: frontmatter.clone(),
        blocks,
    })
}

fn parse_section(
    source: &str,
    kind: RecordKind,
    field: &str,
    lines: &[&str],
) -> Result<Vec<Block>, CliError> {
    let mut content: Vec<&str> = lines.to_vec();
    while content.first().is_some_and(|line| line.trim().is_empty()) {
        content.remove(0);
    }
    while content.last().is_some_and(|line| line.trim().is_empty()) {
        content.pop();
    }
    if content.is_empty() {
        return Ok(Vec::new());
    }
    if content.iter().all(|line| line.starts_with("- ")) && array_field(kind, field) {
        return Ok(vec![Block::List(
            content.iter().map(|line| line[2..].to_owned()).collect(),
        )]);
    }
    if content[0].starts_with("```") {
        let language = content[0][3..].to_owned();
        if content.last().copied() != Some("```") {
            return Err(input_error(
                source,
                kind,
                format!("section '{field}' has an unclosed code block"),
            ));
        }
        let language = (!language.is_empty()).then_some(language);
        return Ok(vec![Block::Code {
            language,
            content: content[1..content.len() - 1].join("\n"),
        }]);
    }
    if content
        .iter()
        .all(|line| line.starts_with('|') && line.ends_with('|'))
    {
        if content.len() < 2 || !content[1].contains("---") {
            return Err(input_error(
                source,
                kind,
                format!("section '{field}' has malformed table headings"),
            ));
        }
        let headers = table_row(content[0]);
        let rows = content[2..].iter().map(|line| table_row(line)).collect();
        return Ok(vec![Block::Table { headers, rows }]);
    }
    if content.len() == 1 {
        if let Some((label, target)) = markdown_link(content[0]) {
            if let Ok(id) = target.parse::<RecordId>() {
                return Ok(vec![Block::RecordReference {
                    id,
                    label: (label != "record").then_some(label),
                }]);
            }
            return Ok(vec![Block::EvidenceReference {
                id: target,
                label: (label != "evidence").then_some(label),
            }]);
        }
    }
    if content.iter().any(|line| line.starts_with('#')) {
        return Err(input_error(
            source,
            kind,
            format!("section '{field}' contains an ambiguous heading"),
        ));
    }
    Ok(vec![Block::Paragraph(
        content
            .join("\n")
            .lines()
            .map(|line| match line.strip_prefix('\\') {
                Some(line) => line,
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )])
}

fn array_field(kind: RecordKind, field: &str) -> bool {
    kind.field_kind(field) == Some(records::FieldKind::List)
}

fn document_to_json(
    source: &str,
    kind: RecordKind,
    document: Document,
) -> Result<JsonValue, CliError> {
    let mut values = Map::new();
    let blocks = document.blocks;
    let mut index = 1;
    for (field, _) in fields(kind) {
        if !matches!(blocks.get(index), Some(Block::Heading { level: 2, .. })) {
            return Err(input_error(
                source,
                kind,
                format!("missing required section '{field}'"),
            ));
        }
        index += 1;
        let start = index;
        while !matches!(blocks.get(index), Some(Block::Heading { level: 2, .. })) {
            if blocks.get(index).is_none() {
                break;
            }
            index += 1;
        }
        values.insert(
            (*field).into(),
            blocks_to_value(blocks[start..index].to_vec()),
        );
    }
    Ok(JsonValue::Object(values))
}

fn blocks_to_value(blocks: Vec<Block>) -> JsonValue {
    if blocks.is_empty() {
        return JsonValue::Array(Vec::new());
    }
    if blocks.len() == 1 {
        return match &blocks[0] {
            Block::Paragraph(value) => match serde_json::from_str(value) {
                Ok(value) => value,
                Err(_) => JsonValue::String(value.clone()),
            },
            Block::List(values) => json!(values),
            Block::Code { language, content } => {
                let mut value = Map::new();
                value.insert("code".into(), content.clone().into());
                if let Some(language) = language {
                    value.insert("language".into(), language.clone().into());
                }
                JsonValue::Object(value)
            }
            Block::Table { headers, rows } => json!({"headers": headers, "rows": rows}),
            Block::Diagram(value) => json!({"diagram": value}),
            Block::RecordReference { id, label } => {
                let mut value = Map::new();
                value.insert("record_reference".into(), id.to_string().into());
                if let Some(label) = label {
                    value.insert("label".into(), label.clone().into());
                }
                JsonValue::Object(value)
            }
            Block::EvidenceReference { id, label } => {
                let mut value = Map::new();
                value.insert("evidence_reference".into(), id.clone().into());
                if let Some(label) = label {
                    value.insert("label".into(), label.clone().into());
                }
                JsonValue::Object(value)
            }
            Block::Heading { .. } => JsonValue::String(String::new()),
        };
    }
    JsonValue::String(String::new())
}

fn table_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split(" | ")
        .map(str::to_owned)
        .collect()
}

fn markdown_link(line: &str) -> Option<(String, String)> {
    let (label, target) = line.strip_prefix('[')?.split_once("](")?;
    Some((label.to_owned(), target.strip_suffix(')')?.to_owned()))
}

fn input_error(source: &str, kind: RecordKind, detail: impl Into<String>) -> CliError {
    CliError::Message(format!("{source} ({kind}): {}", detail.into()))
}

fn source_error(source: &str, kind: RecordKind, error: std::io::Error) -> CliError {
    input_error(source, kind, error.to_string())
}

#[cfg(test)]
mod tests;
