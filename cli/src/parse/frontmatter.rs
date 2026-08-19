use super::input_error;
use crate::document::Frontmatter;
use crate::CliError;
use records::{RecordId, RecordKind, Status, RELATIONSHIPS};
use serde_yaml::{Mapping, Value as YamlValue};
use std::collections::BTreeMap;
use std::str::FromStr;

pub(super) fn parse(source: &str, kind: RecordKind, yaml: &str) -> Result<Frontmatter, CliError> {
    let value: YamlValue = serde_yaml::from_str(yaml)
        .map_err(|error| input_error(source, kind, format!("frontmatter is malformed: {error}")))?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| input_error(source, kind, "frontmatter must be a YAML mapping"))?;
    let allowed = [
        "id",
        "title",
        "record-type",
        "status",
        "revision",
        "date",
        "slug",
        "tags",
        "relationships",
    ];
    for key in mapping.keys() {
        let key = key
            .as_str()
            .ok_or_else(|| input_error(source, kind, "frontmatter field names must be strings"))?;
        if !allowed.contains(&key) {
            return Err(input_error(
                source,
                kind,
                format!("unknown frontmatter field '{key}'"),
            ));
        }
    }
    let id = parse_id(
        source,
        kind,
        required(source, kind, mapping, "id")?.as_str(),
        "id",
    )?;
    if id.kind != kind {
        return Err(input_error(
            source,
            kind,
            format!(
                "frontmatter id has record kind {}, expected {kind}",
                id.kind
            ),
        ));
    }
    let record_type = scalar(source, kind, mapping, "record-type")?;
    if record_type != kind.slug() {
        return Err(input_error(
            source,
            kind,
            format!(
                "frontmatter record-type is '{record_type}', expected '{}'",
                kind.slug()
            ),
        ));
    }
    let status = scalar(source, kind, mapping, "status")?;
    Status::from_str(&status)
        .map_err(|error| input_error(source, kind, format!("invalid status: {error}")))?;
    let revision = required(source, kind, mapping, "revision")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| {
            input_error(
                source,
                kind,
                "frontmatter field 'revision' must be an integer",
            )
        })?;
    let date = scalar(source, kind, mapping, "date")?;
    if date.len() != 10
        || date.as_bytes().get(4) != Some(&b'-')
        || date.as_bytes().get(7) != Some(&b'-')
    {
        return Err(input_error(
            source,
            kind,
            "frontmatter field 'date' must be YYYY-MM-DD",
        ));
    }
    let tags = strings(
        source,
        kind,
        required(source, kind, mapping, "tags")?,
        "tags",
    )?;
    let relationships = relationships(
        source,
        kind,
        required(source, kind, mapping, "relationships")?,
    )?;
    Ok(Frontmatter {
        id,
        title: scalar(source, kind, mapping, "title")?,
        record_type: kind.slug(),
        status,
        revision,
        date,
        slug: scalar(source, kind, mapping, "slug")?,
        tags,
        relationships,
    })
}

fn required<'a>(
    source: &str,
    kind: RecordKind,
    mapping: &'a Mapping,
    name: &str,
) -> Result<&'a YamlValue, CliError> {
    mapping
        .get(YamlValue::String(name.into()))
        .ok_or_else(|| input_error(source, kind, format!("missing frontmatter field '{name}'")))
}

fn scalar(
    source: &str,
    kind: RecordKind,
    mapping: &Mapping,
    name: &str,
) -> Result<String, CliError> {
    required(source, kind, mapping, name)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| {
            input_error(
                source,
                kind,
                format!("frontmatter field '{name}' must be a string"),
            )
        })
}

fn strings(
    source: &str,
    kind: RecordKind,
    value: &YamlValue,
    name: &str,
) -> Result<Vec<String>, CliError> {
    value
        .as_sequence()
        .ok_or_else(|| {
            input_error(
                source,
                kind,
                format!("frontmatter field '{name}' must be a sequence"),
            )
        })?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                input_error(
                    source,
                    kind,
                    format!("frontmatter field '{name}' must contain only strings"),
                )
            })
        })
        .collect()
}

fn relationships(
    source: &str,
    kind: RecordKind,
    value: &YamlValue,
) -> Result<BTreeMap<String, Vec<RecordId>>, CliError> {
    let mapping = value.as_mapping().ok_or_else(|| {
        input_error(
            source,
            kind,
            "frontmatter field 'relationships' must be a mapping",
        )
    })?;
    let mut result = BTreeMap::new();
    for (relation, targets) in mapping {
        let relation = relation
            .as_str()
            .ok_or_else(|| input_error(source, kind, "relationship names must be strings"))?;
        if !RELATIONSHIPS.contains(&relation) {
            return Err(input_error(
                source,
                kind,
                format!("unknown relationship '{relation}'"),
            ));
        }
        let targets = strings(source, kind, targets, "relationships")?;
        let targets = targets
            .into_iter()
            .map(|target| parse_id(source, kind, Some(&target), "relationship target"))
            .collect::<Result<Vec<_>, _>>()?;
        result.insert(relation.into(), targets);
    }
    Ok(result)
}

fn parse_id(
    source: &str,
    kind: RecordKind,
    value: Option<&str>,
    name: &str,
) -> Result<RecordId, CliError> {
    let value = value.ok_or_else(|| {
        input_error(
            source,
            kind,
            format!("frontmatter field '{name}' must be a string"),
        )
    })?;
    value.parse().map_err(|error| {
        input_error(
            source,
            kind,
            format!("frontmatter field '{name}' is invalid: {error}"),
        )
    })
}
