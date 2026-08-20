use crate::CliError;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) struct StagedFile {
    pub(crate) publication_path: String,
    pub(crate) contents: Vec<u8>,
}

pub(crate) fn publish(target: &Path, files: &[StagedFile]) -> Result<(), CliError> {
    check_collisions(files)?;

    let parent = target.parent().ok_or_else(|| {
        CliError::Message(format!(
            "publish target has no parent directory: {}",
            target.display()
        ))
    })?;
    let name = target.file_name().ok_or_else(|| {
        CliError::Message(format!(
            "publish target has no file name: {}",
            target.display()
        ))
    })?;
    let staging = parent.join(format!(
        "{}.staging-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    fs::create_dir(&staging)?;

    for file in files {
        let relative_path = Path::new(&file.publication_path);
        validate_relative_path(relative_path, &file.publication_path)?;
        let path = staging.join(relative_path);
        if let Some(file_parent) = path.parent() {
            fs::create_dir_all(file_parent)?;
        }
        fs::write(path, &file.contents)?;
    }

    validate_staging(&staging, files)?;
    replace_target(target, &staging)
}

fn check_collisions(files: &[StagedFile]) -> Result<(), CliError> {
    let mut paths = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        if let Some(previous) = paths.insert(&file.publication_path, index) {
            return Err(CliError::Message(format!(
                "duplicate publication path {:?} at entries {} and {}",
                file.publication_path, previous, index
            )));
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path, display: &str) -> Result<(), CliError> {
    if path.as_os_str().is_empty() || !path.is_relative() {
        return Err(CliError::Message(format!(
            "publication path must be non-empty and relative: {display:?}"
        )));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(CliError::Message(format!(
            "publication path must not contain '..': {display:?}"
        )));
    }
    Ok(())
}

fn validate_staging(staging: &Path, files: &[StagedFile]) -> Result<(), CliError> {
    for file in files {
        let path = staging.join(&file.publication_path);
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file() || metadata.len() != file.contents.len() as u64 {
            return Err(CliError::Message(format!(
                "staged file has unexpected contents: {}",
                file.publication_path
            )));
        }
        if fs::read(&path)? != file.contents {
            return Err(CliError::Message(format!(
                "staged file contents changed: {}",
                file.publication_path
            )));
        }
    }
    Ok(())
}

fn replace_target(target: &Path, staging: &Path) -> Result<(), CliError> {
    if !target.exists() {
        fs::rename(staging, target)?;
        return Ok(());
    }

    if !target.is_dir() {
        return Err(CliError::Message(format!(
            "publish target is not a directory: {}",
            target.display()
        )));
    }

    let backup = PathBuf::from(format!("{}.old-{}", target.display(), std::process::id()));
    // Linux cannot rename a directory over a non-empty directory. Move the old
    // target aside, swap the staged directory into its name, then remove the
    // old tree. If the swap fails, restore the old target and leave staging.
    fs::rename(target, &backup)?;
    if let Err(error) = fs::rename(staging, target) {
        let restore = fs::rename(&backup, target);
        return match restore {
            Ok(()) => Err(error.into()),
            Err(restore_error) => Err(CliError::Message(format!(
                "could not replace publish target: {error}; could not restore old target: {restore_error}"
            ))),
        };
    }

    if let Err(error) = fs::remove_dir_all(&backup) {
        let rollback = fs::rename(target, staging).and_then(|()| fs::rename(&backup, target));
        return match rollback {
            Ok(()) => Err(error.into()),
            Err(rollback_error) => Err(CliError::Message(format!(
                "could not remove old publish target: {error}; could not restore it: {rollback_error}"
            ))),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        directory: PathBuf,
        target: PathBuf,
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
        let directory = std::env::temp_dir().join(format!("strata-stage-test-{suffix}"));
        fs::create_dir_all(&directory).expect("temp directory");
        let target = directory.join("site");
        Fixture { directory, target }
    }

    fn file(path: &str, contents: &[u8]) -> StagedFile {
        StagedFile {
            publication_path: path.into(),
            contents: contents.into(),
        }
    }

    fn staging_path(target: &Path) -> PathBuf {
        target.parent().expect("target parent").join(format!(
            "{}.staging-{}",
            target.file_name().expect("target name").to_string_lossy(),
            std::process::id()
        ))
    }

    #[test]
    fn publishes_complete_staged_content() {
        let fixture = fixture();
        publish(
            &fixture.target,
            &[
                file("adr/0001-page.html", b"page"),
                file("index.html", b"index"),
            ],
        )
        .expect("publish");
        assert_eq!(
            fs::read(fixture.target.join("adr/0001-page.html")).expect("page"),
            b"page"
        );
        assert_eq!(
            fs::read(fixture.target.join("index.html")).expect("index"),
            b"index"
        );
        assert!(!staging_path(&fixture.target).exists());
    }

    #[test]
    fn rejects_collision_without_touching_filesystem() {
        let fixture = fixture();
        let error = publish(
            &fixture.target,
            &[file("index.html", b"one"), file("index.html", b"two")],
        )
        .expect_err("collision");
        assert!(error.to_string().contains("duplicate publication path"));
        assert!(!fixture.target.exists());
        assert!(!staging_path(&fixture.target).exists());
    }

    #[test]
    fn replaces_old_content_instead_of_merging() {
        let fixture = fixture();
        fs::create_dir_all(fixture.target.join("old")).expect("old directory");
        fs::write(fixture.target.join("old/stale.html"), b"stale").expect("stale file");
        publish(&fixture.target, &[file("fresh.html", b"fresh")]).expect("publish");
        assert_eq!(
            fs::read(fixture.target.join("fresh.html")).expect("fresh"),
            b"fresh"
        );
        assert!(!fixture.target.join("old/stale.html").exists());
    }

    #[test]
    fn staging_failure_leaves_partial_stage_and_old_target() {
        let fixture = fixture();
        fs::create_dir_all(&fixture.target).expect("target directory");
        fs::write(fixture.target.join("old.html"), b"old").expect("old file");
        let error = publish(
            &fixture.target,
            &[file("good.html", b"good"), file("../outside.html", b"bad")],
        )
        .expect_err("invalid path");
        assert!(error.to_string().contains("must not contain"));
        assert_eq!(
            fs::read(fixture.target.join("old.html")).expect("old"),
            b"old"
        );
        assert!(!fixture.target.join("good.html").exists());
        assert_eq!(
            fs::read(staging_path(&fixture.target).join("good.html")).expect("partial"),
            b"good"
        );
    }
}
