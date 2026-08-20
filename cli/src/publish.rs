use crate::assets;
use crate::config::ResolvedConfig;
use crate::manifest::{Manifest, ManifestEntry};
use crate::render_backend::{self, RenderJob};
use crate::stage::{self, StagedFile};
use crate::{validate, CliError};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use store::Store;

#[derive(Debug, Serialize)]
pub(crate) struct PublishOutcome {
    pub(crate) clean: bool,
    pub(crate) issues: Vec<String>,
    #[serde(skip)]
    pub(crate) published: usize,
}

pub(crate) fn run(
    store: &Store,
    check: bool,
    config: &ResolvedConfig,
) -> Result<PublishOutcome, CliError> {
    let issues = validate::issues(store, config)?;
    let errors = issues
        .iter()
        .filter(|issue| !issue.starts_with("WARN "))
        .cloned()
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(CliError::Message(format!(
            "publish blocked by validation error: {}",
            errors.join("; ")
        )));
    }

    let manifest = crate::manifest::build(store, config)?;
    if check {
        return check_manifest(&manifest, &config.publish_target.value);
    }

    let scratch = ScratchDirectory::new()?;
    let template = assets::write_template(scratch.path())?;
    let mut files = Vec::with_capacity(manifest.entries.len() + 2);
    for (index, entry) in manifest.entries.iter().enumerate() {
        files.push(render_entry(
            scratch.path(),
            &template,
            index,
            entry,
            &config.publish_renderer.value,
        )?);
    }
    files.push(StagedFile {
        publication_path: "manifest.json".into(),
        contents: serde_json::to_vec(&manifest)?,
    });
    files.push(StagedFile {
        publication_path: Path::new(assets::ASSETS_DIR_NAME)
            .join(assets::STYLESHEET_FILE_NAME)
            .display()
            .to_string(),
        contents: assets::STYLESHEET.as_bytes().to_vec(),
    });
    stage::publish(&config.publish_target.value, &files)?;
    Ok(PublishOutcome {
        clean: true,
        issues: Vec::new(),
        published: manifest.entries.len(),
    })
}

fn render_entry(
    scratch: &Path,
    template: &Path,
    index: usize,
    entry: &ManifestEntry,
    renderer: &str,
) -> Result<StagedFile, CliError> {
    let input = scratch.join(format!("input-{index}.md"));
    let output = scratch.join(format!("output-{index}.html"));
    let metadata = scratch.join(format!("metadata-{index}.json"));
    fs::write(&input, &entry.rendered_markdown)?;
    let css = PathBuf::from("..")
        .join(assets::ASSETS_DIR_NAME)
        .join(assets::STYLESHEET_FILE_NAME);
    let contents = render_backend::render(
        renderer,
        &RenderJob {
            input: &input,
            output: &output,
            metadata: &metadata,
            template,
            css: &css,
            entry,
        },
    )?;
    Ok(StagedFile {
        publication_path: entry.publication_path.clone(),
        contents,
    })
}

fn check_manifest(manifest: &Manifest, target: &Path) -> Result<PublishOutcome, CliError> {
    let path = target.join("manifest.json");
    let old = match fs::read(&path) {
        Ok(contents) => serde_json::from_slice::<Manifest>(&contents)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Manifest {
            schema_version: manifest.schema_version,
            generated_at: String::new(),
            strata_build: String::new(),
            source_database: String::new(),
            renderer_command: String::new(),
            entries: Vec::new(),
        },
        Err(error) => return Err(error.into()),
    };
    let issues = manifest_differences(manifest, &old);
    Ok(PublishOutcome {
        clean: issues.is_empty(),
        issues,
        published: 0,
    })
}

fn manifest_differences(current: &Manifest, old: &Manifest) -> Vec<String> {
    let current_by_id = current
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let old_by_id = old
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut issues = Vec::new();
    for (id, entry) in &current_by_id {
        match old_by_id.get(id) {
            None => issues.push(format!("missing {}", entry.publication_path)),
            Some(previous) if previous.publication_path != entry.publication_path => {
                issues.push(format!("stale {}", previous.publication_path));
                issues.push(format!("missing {}", entry.publication_path));
            }
            Some(previous) if previous.content_hash != entry.content_hash => {
                issues.push(format!("different {}", entry.publication_path));
            }
            Some(_) => {}
        }
    }
    for (id, entry) in &old_by_id {
        if !current_by_id.contains_key(id) {
            issues.push(format!("stale {}", entry.publication_path));
        }
    }
    issues.sort();
    issues
}

struct ScratchDirectory {
    path: PathBuf,
}

impl ScratchDirectory {
    fn new() -> Result<Self, CliError> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| {
                CliError::Message(format!("system clock is before Unix epoch: {error}"))
            })?
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("strata-publish-{}-{nonce}", std::process::id()));
        fs::create_dir(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ResolvedPath, ResolvedValue, Source};
    use records::RecordKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        directory: PathBuf,
        store: Store,
        config: ResolvedConfig,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).expect("temporary fixture cleanup");
        }
    }

    fn fixture() -> Fixture {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("strata-publish-test-{suffix}"));
        fs::create_dir_all(&directory).expect("temporary fixture");
        let export_target = directory.join("records");
        let dump_target = directory.join("snapshot.sql");
        let target = directory.join("site");
        let mut store = Store::open_memory().expect("store");
        store
            .create(
                RecordKind::Adr,
                "Publish test",
                RecordKind::Adr.default_document("Publish test"),
            )
            .expect("record");
        let config = ResolvedConfig {
            database: ResolvedPath {
                value: directory.join("strata.db"),
                source: Source::Default,
            },
            export_target: ResolvedPath {
                value: export_target,
                source: Source::Default,
            },
            dump_target: ResolvedPath {
                value: dump_target,
                source: Source::Default,
            },
            publish_target: ResolvedPath {
                value: target,
                source: Source::Default,
            },
            publish_renderer: ResolvedValue {
                value: "sh -c 'cat \"$1\" > \"$2\"' sh {input} {output}".into(),
                source: Source::Default,
            },
        };
        crate::export::run(&store, false, &config.export_target.value).expect("export");
        crate::dump::run(&store, false, &config.dump_target.value).expect("dump");
        Fixture {
            directory,
            store,
            config,
        }
    }

    #[test]
    fn real_publish_writes_pages_manifest_and_stylesheet() {
        let fixture = fixture();
        let outcome = run(&fixture.store, false, &fixture.config).expect("publish");
        assert!(outcome.clean);
        assert!(fixture
            .config
            .publish_target
            .value
            .join("adr/0001-publish-test.html")
            .is_file());
        assert!(fixture
            .config
            .publish_target
            .value
            .join("manifest.json")
            .is_file());
        assert!(fixture
            .config
            .publish_target
            .value
            .join("_assets/style.css")
            .is_file());
    }

    #[test]
    fn check_missing_manifest_does_not_write_target() {
        let fixture = fixture();
        let outcome = run(&fixture.store, true, &fixture.config).expect("check");
        assert!(!outcome.clean);
        assert_eq!(outcome.issues, vec!["missing adr/0001-publish-test.html"]);
        assert!(!fixture.config.publish_target.value.exists());
    }

    #[test]
    fn check_matching_manifest_is_clean() {
        let fixture = fixture();
        run(&fixture.store, false, &fixture.config).expect("publish");
        let outcome = run(&fixture.store, true, &fixture.config).expect("check");
        assert!(outcome.clean);
        assert!(outcome.issues.is_empty());
    }

    #[test]
    fn check_reports_changed_content() {
        let mut fixture = fixture();
        run(&fixture.store, false, &fixture.config).expect("publish");
        let record = fixture.store.all_records().expect("record").remove(0);
        let mut document = record.document;
        document["decision"] = serde_json::Value::String("Changed".into());
        fixture
            .store
            .revise(&record.id, document, None)
            .expect("revise");
        crate::export::run(&fixture.store, false, &fixture.config.export_target.value)
            .expect("export");
        crate::dump::run(&fixture.store, false, &fixture.config.dump_target.value).expect("dump");
        let outcome = run(&fixture.store, true, &fixture.config).expect("check");
        assert_eq!(outcome.issues, vec!["different adr/0001-publish-test.html"]);
    }

    #[test]
    fn validation_error_prevents_publish_writes() {
        let mut fixture = fixture();
        let record = fixture.store.all_records().expect("record").remove(0);
        fixture
            .store
            .set_status(&record.id, records::Status::Proposed)
            .expect("propose");
        fixture
            .store
            .set_status(&record.id, records::Status::Accepted)
            .expect("accept");
        fixture
            .store
            .create(
                RecordKind::Adr,
                "Superseding record",
                RecordKind::Adr.default_document("Superseding record"),
            )
            .expect("source");
        let source = fixture.store.all_records().expect("records")[1].id.clone();
        fixture
            .store
            .link(&source, "supersedes", &record.id)
            .expect("relationship");
        let error = run(&fixture.store, false, &fixture.config).expect_err("validation error");
        assert!(error.to_string().contains("remains accepted"));
        assert!(!fixture.config.publish_target.value.exists());
    }
}
