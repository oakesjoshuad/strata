use crate::render::{render_aggregate, render_record};
use crate::CliError;
use records::{ExportLayout, Record, RecordKind};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use store::Store;

#[derive(Debug, Eq, PartialEq)]
enum Difference {
    Missing(PathBuf),
    Stale(PathBuf),
    Different(PathBuf),
}

pub(crate) fn run(store: &Store, check: bool, root: &Path) -> Result<(), CliError> {
    let expected = rendered_records(store, root)?;
    let differences = differences(&expected, root)?;

    if check {
        if differences.is_empty() {
            println!("export check clean");
            return Ok(());
        }
        for difference in differences {
            println!("{}", difference_message(&difference));
        }
        return Err(CliError::Message("export check failed".into()));
    }

    write_projection(&expected, root)?;
    println!("exported {} records", store.all_records()?.len());
    Ok(())
}

pub(crate) fn stale_messages(store: &Store, root: &Path) -> Result<Vec<String>, CliError> {
    let expected = rendered_records(store, root)?;
    Ok(differences(&expected, root)?
        .iter()
        .map(difference_message)
        .collect())
}

pub(crate) fn rendered_records(
    store: &Store,
    root: &Path,
) -> Result<BTreeMap<PathBuf, String>, CliError> {
    let records = store.all_records()?;
    let mut rendered = BTreeMap::new();
    let mut aggregate_records = BTreeMap::<RecordKind, Vec<&Record>>::new();
    for record in &records {
        match record.id.kind.export_layout() {
            ExportLayout::PerRecord => {
                rendered.insert(record_path(root, record), render_record(store, &record.id)?);
            }
            ExportLayout::Aggregate { .. } => {
                aggregate_records
                    .entry(record.id.kind)
                    .or_default()
                    .push(record);
            }
        }
    }
    for (kind, records) in aggregate_records {
        let ExportLayout::Aggregate { file } = kind.export_layout() else {
            continue;
        };
        rendered.insert(root.join(file), render_aggregate(kind, &records)?);
    }
    Ok(rendered)
}

pub(crate) fn record_path(root: &Path, record: &Record) -> PathBuf {
    root.join(record.id.kind.slug())
        .join(format!("{:04}-{}.md", record.id.number, record.slug))
}

fn differences(
    expected: &BTreeMap<PathBuf, String>,
    root: &Path,
) -> Result<Vec<Difference>, CliError> {
    let mut differences = Vec::new();
    for (path, content) in expected {
        match fs::read(path) {
            Ok(on_disk) if on_disk == content.as_bytes() => {}
            Ok(_) => differences.push(Difference::Different(path.clone())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                differences.push(Difference::Missing(path.clone()))
            }
            Err(error) => return Err(error.into()),
        }
    }

    for path in owned_paths(root) {
        if path.is_file() {
            if !expected.contains_key(&path) {
                differences.push(Difference::Stale(path));
            }
            continue;
        }
        let directory = path;
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            if !expected.contains_key(&path) {
                differences.push(Difference::Stale(path));
            }
        }
    }
    differences.sort_by(|left, right| difference_path(left).cmp(difference_path(right)));
    Ok(differences)
}

fn difference_path(difference: &Difference) -> &Path {
    match difference {
        Difference::Missing(path) | Difference::Stale(path) | Difference::Different(path) => path,
    }
}

fn difference_message(difference: &Difference) -> String {
    match difference {
        Difference::Missing(path) => format!("missing {}", path.display()),
        Difference::Stale(path) => format!("stale {}", path.display()),
        Difference::Different(path) => format!("different {}", path.display()),
    }
}

fn write_projection(expected: &BTreeMap<PathBuf, String>, root: &Path) -> Result<(), CliError> {
    for (path, content) in expected {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
    }
    for path in owned_paths(root) {
        if path.is_file() {
            if !expected.contains_key(&path) {
                fs::remove_file(path)?;
            }
            continue;
        }
        let directory = path;
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_file() && !expected.contains_key(&path) {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

fn owned_paths(root: &Path) -> Vec<PathBuf> {
    RecordKind::ALL
        .into_iter()
        .map(|kind| match kind.export_layout() {
            ExportLayout::PerRecord => root.join(kind.slug()),
            ExportLayout::Aggregate { file } => root.join(file),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::RecordKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        directory: PathBuf,
        store: Store,
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).expect("remove temporary fixture");
        }
    }

    fn fixture() -> Fixture {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("strata-export-test-{suffix}"));
        fs::create_dir_all(&directory).expect("temp directory");
        let database = directory.join("test.db");
        let mut store = Store::open(&database).expect("store");
        store
            .create(
                RecordKind::Adr,
                "Export test",
                RecordKind::Adr.default_document("Export test"),
            )
            .expect("record");
        let root = directory.join("docs/records");
        fs::create_dir_all(root.join("adr")).expect("records directory");
        Fixture {
            directory,
            store,
            root,
        }
    }

    #[test]
    fn check_reports_missing_file() {
        let fixture = fixture();
        let expected = rendered_records(&fixture.store, &fixture.root).expect("render");
        let differences = differences(&expected, &fixture.root).expect("check");
        assert!(differences
            .iter()
            .any(|difference| matches!(difference, Difference::Missing(_))));
    }

    #[test]
    fn check_reports_stale_file() {
        let fixture = fixture();
        let stale = fixture.root.join("adr/9999-stale.md");
        fs::write(&stale, "stale").expect("stale file");
        let expected = rendered_records(&fixture.store, &fixture.root).expect("render");
        let differences = differences(&expected, &fixture.root).expect("check");
        assert!(differences
            .iter()
            .any(|difference| matches!(difference, Difference::Stale(path) if path == &stale)));
    }

    #[test]
    fn check_reports_different_file() {
        let fixture = fixture();
        let expected = rendered_records(&fixture.store, &fixture.root).expect("render");
        let path = expected.keys().next().expect("expected path");
        fs::write(path, b"same content except bytes").expect("different file");
        let differences = differences(&expected, &fixture.root).expect("check");
        assert!(differences
            .iter()
            .any(|difference| matches!(difference, Difference::Different(found) if found == path)));
    }

    #[test]
    fn renders_aggregate_glossary_and_risk_projections() {
        let mut fixture = fixture();
        fixture
            .store
            .create(
                RecordKind::Glossary,
                "Lead",
                RecordKind::Glossary.default_document("Lead"),
            )
            .expect("glossary");
        fixture
            .store
            .create(
                RecordKind::Risk,
                "Index stale",
                RecordKind::Risk.default_document("Index stale"),
            )
            .expect("risk");

        let rendered = rendered_records(&fixture.store, &fixture.root).expect("render");
        let glossary = fixture.root.join("glossary.md");
        let risks = fixture.root.join("risk-register.md");
        assert!(rendered.contains_key(&glossary));
        assert!(rendered.contains_key(&risks));
        assert!(rendered[&glossary].contains("{#0001-lead}"));
        assert!(rendered[&risks].contains("| ID | Title | Status |"));
    }
}
