use crate::config::ResolvedConfig;
use crate::export::{record_path, rendered_records};
use crate::render::render_record;
use crate::{build_version, CliError};
use records::{ExportLayout, Record};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::path::Path;
use store::Store;

/// The version of the record-derived publication manifest schema.
pub(crate) const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Manifest {
    pub(crate) schema_version: u32,
    pub(crate) generated_at: String,
    pub(crate) strata_build: String,
    pub(crate) source_database: String,
    pub(crate) renderer_command: String,
    pub(crate) entries: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ManifestEntry {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) revision: u32,
    pub(crate) slug: String,
    pub(crate) publication_path: String,
    pub(crate) anchor: Option<String>,
    pub(crate) title: String,
    pub(crate) tags: Vec<String>,
    pub(crate) content_hash: String,
    pub(crate) relationships: Vec<ResolvedRelationship>,
    // The Markdown is needed for rendering but is intentionally not persisted
    // in manifest.json; the hash is the persisted projection fingerprint.
    #[serde(skip)]
    pub(crate) rendered_markdown: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ResolvedRelationship {
    pub(crate) relation: String,
    pub(crate) target_id: String,
    pub(crate) target_title: String,
    pub(crate) target_href: String,
}

pub(crate) fn build(store: &Store, config: &ResolvedConfig) -> Result<Manifest, CliError> {
    let records = store.all_records()?;
    let rendered = rendered_records(store, &config.export_target.value)?;
    let mut entries = records
        .iter()
        .map(|record| manifest_entry(store, record, &rendered, &config.export_target.value))
        .collect::<Result<Vec<_>, _>>()?;

    // Resolve graph edges from the entries already built above. This keeps
    // publication assembly to one record query rather than re-querying each
    // relationship target for its title and persisted publication path.
    let index = entries
        .iter()
        .map(|entry| {
            (
                entry.id.clone(),
                (entry.title.clone(), publication_href(entry)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (record, entry) in records.iter().zip(entries.iter_mut()) {
        entry.relationships = store
            .outgoing_relationships(&record.id)?
            .into_iter()
            .map(|relationship| {
                let target_id = relationship.target_id.to_string();
                let (target_title, target_href) = index.get(&target_id).ok_or_else(|| {
                    CliError::Message(format!(
                        "relationship target is missing from manifest: {}",
                        target_id
                    ))
                })?;
                Ok(ResolvedRelationship {
                    relation: relationship.relation,
                    target_id,
                    target_title: target_title.clone(),
                    target_href: target_href.clone(),
                })
            })
            .collect::<Result<Vec<_>, CliError>>()?;
    }

    Ok(Manifest {
        schema_version: SCHEMA_VERSION,
        generated_at: chrono::Utc::now().to_rfc3339(),
        strata_build: build_version().to_owned(),
        source_database: config.database.value.display().to_string(),
        renderer_command: config.publish_renderer.value.clone(),
        entries,
    })
}

fn manifest_entry(
    store: &Store,
    record: &Record,
    rendered: &std::collections::BTreeMap<std::path::PathBuf, String>,
    export_root: &Path,
) -> Result<ManifestEntry, CliError> {
    let rendered_path = rendered_path(export_root, record);
    let markdown = rendered.get(&rendered_path).ok_or_else(|| {
        CliError::Message(format!(
            "rendered record is missing from export result: {}",
            rendered_path.display()
        ))
    })?;
    let (publication_path, anchor) = publication_location(record);
    Ok(ManifestEntry {
        id: record.id.to_string(),
        kind: record.id.kind.to_string(),
        status: record.status.to_string(),
        revision: record.revision,
        slug: record.slug.clone(),
        publication_path,
        anchor,
        title: record.title.clone(),
        tags: tags(record)?,
        content_hash: content_hash(&render_record(store, &record.id)?),
        relationships: Vec::new(),
        rendered_markdown: markdown.clone(),
    })
}

fn rendered_path(export_root: &Path, record: &Record) -> std::path::PathBuf {
    match record.id.kind.export_layout() {
        ExportLayout::PerRecord => record_path(export_root, record),
        ExportLayout::Aggregate { file } => export_root.join(file),
    }
}

fn publication_location(record: &Record) -> (String, Option<String>) {
    match record.id.kind.export_layout() {
        ExportLayout::PerRecord => (
            format!(
                "{}/{:04}-{}.html",
                record.id.kind.slug(),
                record.id.number,
                record.slug
            ),
            None,
        ),
        ExportLayout::Aggregate { file } => (
            file.replace(".md", ".html"),
            Some(format!("{:04}-{}", record.id.number, record.slug)),
        ),
    }
}

fn publication_href(entry: &ManifestEntry) -> String {
    match &entry.anchor {
        Some(anchor) => format!("../{}#{anchor}", entry.publication_path),
        None => format!("../{}", entry.publication_path),
    }
}

fn tags(record: &Record) -> Result<Vec<String>, CliError> {
    let Some(value) = record.document.get("tags") else {
        return Ok(Vec::new());
    };
    let JsonValue::Array(values) = value else {
        return Err(CliError::Message("record tags must be a JSON array".into()));
    };
    values
        .iter()
        .map(|value| match value {
            JsonValue::String(value) => Ok(value.clone()),
            value => Ok(serde_json::to_string(value)?),
        })
        .collect()
}

fn content_hash(markdown: &str) -> String {
    blake3::hash(markdown.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ResolvedPath, ResolvedValue, Source};
    use records::RecordKind;
    use std::path::PathBuf;

    fn config() -> ResolvedConfig {
        ResolvedConfig {
            database: ResolvedPath {
                value: PathBuf::from("/tmp/strata-test.db"),
                source: Source::Default,
            },
            export_target: ResolvedPath {
                value: PathBuf::from("/tmp/strata-export"),
                source: Source::Default,
            },
            dump_target: ResolvedPath {
                value: PathBuf::from("/tmp/strata-dump.sql"),
                source: Source::Default,
            },
            publish_target: ResolvedPath {
                value: PathBuf::from("/tmp/strata-site"),
                source: Source::Default,
            },
            publish_renderer: ResolvedValue {
                value: "renderer {input}".into(),
                source: Source::Default,
            },
            code_ref_locator: ResolvedValue {
                value: "graphlite".into(),
                source: Source::Default,
            },
        }
    }

    fn fixture() -> (Store, ResolvedConfig) {
        let mut store = Store::open_memory().expect("store");
        store
            .create(
                RecordKind::Adr,
                "Manifest record",
                RecordKind::Adr.default_document("Manifest record"),
            )
            .expect("record");
        (store, config())
    }

    #[test]
    fn manifest_contains_record_fields_and_is_stable() {
        let (store, config) = fixture();
        let first = build(&store, &config).expect("manifest");
        let second = build(&store, &config).expect("manifest");
        assert_eq!(first.entries.len(), 1);
        assert_eq!(first.entries[0].id, "ADR-0001");
        assert_eq!(
            first.entries[0].publication_path,
            "adr/0001-manifest-record.html"
        );
        assert_eq!(
            first.entries[0].content_hash,
            second.entries[0].content_hash
        );
        assert_eq!(first.entries[0].tags, Vec::<String>::new());
        assert!(first.entries[0].relationships.is_empty());
        assert_eq!(first.schema_version, SCHEMA_VERSION);
        assert_eq!(first.strata_build, build_version());
    }

    #[test]
    fn content_hash_changes_when_rendered_markdown_changes() {
        let (mut store, config) = fixture();
        let record = store.all_records().expect("record").remove(0);
        let first = build(&store, &config).expect("manifest");
        let mut document = record.document;
        document["decision"] = JsonValue::String("A changed decision.".into());
        store.revise(&record.id, document, None).expect("revision");
        let second = build(&store, &config).expect("manifest");
        assert_ne!(
            first.entries[0].content_hash,
            second.entries[0].content_hash
        );
    }

    #[test]
    fn resolves_outgoing_relationships_against_manifest_entries() {
        let (mut store, config) = fixture();
        let target = store
            .create(
                RecordKind::Adr,
                "Relationship target",
                RecordKind::Adr.default_document("Relationship target"),
            )
            .expect("target");
        let source = store.all_records().expect("records")[0].id.clone();
        store
            .link(&source, "relates-to", &target.id)
            .expect("relationship");

        let manifest = build(&store, &config).expect("manifest");
        let relationship = &manifest.entries[0].relationships[0];
        assert_eq!(relationship.relation, "relates-to");
        assert_eq!(relationship.target_id, "ADR-0002");
        assert_eq!(relationship.target_title, "Relationship target");
        assert_eq!(
            relationship.target_href,
            "../adr/0002-relationship-target.html"
        );
    }

    #[test]
    fn aggregate_entries_share_a_page_and_keep_a_stable_anchor() {
        let (mut store, config) = fixture();
        let glossary = store
            .create(
                RecordKind::Glossary,
                "Lead",
                RecordKind::Glossary.default_document("Lead"),
            )
            .expect("glossary");
        let source = store.all_records().expect("records")[0].id.clone();
        store
            .link(&source, "uses-term", &glossary.id)
            .expect("relationship");

        let manifest = build(&store, &config).expect("manifest");
        let glossary = manifest
            .entries
            .iter()
            .find(|entry| entry.id == "GLOS-0001")
            .expect("glossary entry");
        assert_eq!(glossary.publication_path, "glossary.html");
        assert_eq!(glossary.anchor.as_deref(), Some("0001-lead"));
        assert_eq!(
            manifest.entries[0].relationships[0].target_href,
            "../glossary.html#0001-lead"
        );
    }
}
