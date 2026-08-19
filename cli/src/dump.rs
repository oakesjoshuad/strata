use crate::CliError;
use std::fs;
use std::path::{Path, PathBuf};
use store::Store;

#[derive(Debug, Eq, PartialEq)]
enum Difference {
    Missing(PathBuf),
    Different(PathBuf),
}

pub(crate) enum DumpResult {
    Checked,
    Written(usize),
}

pub(crate) fn run(store: &Store, check: bool, path: &Path) -> Result<DumpResult, CliError> {
    let expected = store.dump()?;
    let difference = difference(&expected, path)?;
    if check {
        if let Some(difference) = difference {
            return Err(CliError::Message(format!(
                "dump check failed: {}",
                difference_message(&difference)
            )));
        }
        return Ok(DumpResult::Checked);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, expected.as_bytes())?;
    Ok(DumpResult::Written(expected.len()))
}

pub(crate) fn restore(store: &mut Store, path: &Path) -> Result<(), CliError> {
    let dump = fs::read_to_string(path)?;
    store.restore(&dump)?;
    Ok(())
}

fn difference(expected: &str, path: &Path) -> Result<Option<Difference>, CliError> {
    match fs::read(path) {
        Ok(on_disk) if on_disk == expected.as_bytes() => Ok(None),
        Ok(_) => Ok(Some(Difference::Different(path.to_path_buf()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Some(Difference::Missing(path.to_path_buf())))
        }
        Err(error) => Err(error.into()),
    }
}

fn difference_message(difference: &Difference) -> String {
    match difference {
        Difference::Missing(path) => format!("missing {}", path.display()),
        Difference::Different(path) => format!("different {}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::RecordKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        directory: PathBuf,
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
        let directory = std::env::temp_dir().join(format!("strata-dump-test-{suffix}"));
        fs::create_dir_all(&directory).expect("temp directory");
        let mut store = Store::open_memory().expect("store");
        store
            .create(
                RecordKind::Adr,
                "Dump test",
                RecordKind::Adr.default_document("Dump test"),
            )
            .expect("record");
        Fixture { directory }
    }

    #[test]
    fn check_reports_missing_file() {
        let fixture = fixture();
        let path = fixture.directory.join("docs/db-snapshot.sql");
        let difference = difference("expected", &path).expect("check");
        assert!(matches!(difference, Some(Difference::Missing(found)) if found == path));
    }

    #[test]
    fn check_reports_stale_file() {
        let fixture = fixture();
        let path = fixture.directory.join("snapshot.sql");
        fs::write(&path, "stale").expect("stale file");
        let difference = difference("expected", &path).expect("check");
        assert!(matches!(difference, Some(Difference::Different(found)) if found == path));
    }

    #[test]
    fn check_reports_different_file() {
        let fixture = fixture();
        let path = fixture.directory.join("different.sql");
        fs::write(&path, "different").expect("different file");
        let difference = difference("expected", &path).expect("check");
        assert!(matches!(difference, Some(Difference::Different(found)) if found == path));
    }
}
