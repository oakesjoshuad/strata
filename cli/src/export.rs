use crate::render::render_record;
use crate::CliError;
use records::Record;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use store::Store;

const RECORD_DIRECTORIES: [&str; 4] = ["rfc", "pdr", "adr", "edr"];

#[derive(Debug, Eq, PartialEq)]
enum Difference {
    Missing(PathBuf),
    Stale(PathBuf),
    Different(PathBuf),
}

pub(crate) fn run(store: &Store, check: bool) -> Result<(), CliError> {
    let root = Path::new("docs/records");
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
    println!("exported {} records", expected.len());
    Ok(())
}

fn rendered_records(store: &Store, root: &Path) -> Result<BTreeMap<PathBuf, String>, CliError> {
    store
        .all_records()?
        .iter()
        .map(|record| {
            let path = record_path(root, record);
            let rendered = render_record(store, &record.id)?;
            Ok((path, rendered))
        })
        .collect()
}

fn record_path(root: &Path, record: &Record) -> PathBuf {
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

    for kind in RECORD_DIRECTORIES {
        let directory = root.join(kind);
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
    for kind in RECORD_DIRECTORIES {
        let directory = root.join(kind);
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
}
