use crate::CliError;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const DATABASE_DEFAULT: &str = ".strata/strata.db";
pub(crate) const EXPORT_TARGET_DEFAULT: &str = "docs/records";
pub(crate) const DUMP_TARGET_DEFAULT: &str = "docs/db-snapshot.sql";
const CONFIG_FILE: &str = "strata.config.json";

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Source {
    Cli,
    Env,
    File,
    Default,
}

impl Source {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Env => "env",
            Self::File => "file",
            Self::Default => "default",
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResolvedPath {
    pub(crate) value: PathBuf,
    pub(crate) source: Source,
}

#[derive(Debug)]
pub(crate) struct ResolvedConfig {
    pub(crate) database: ResolvedPath,
    pub(crate) export_target: ResolvedPath,
    pub(crate) dump_target: ResolvedPath,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    database: Option<String>,
    export_target: Option<String>,
    dump_target: Option<String>,
}

struct Inputs<'a> {
    database: Option<&'a Path>,
    export_target: Option<&'a Path>,
    dump_target: Option<&'a Path>,
}

pub(crate) fn resolve(
    database: Option<&Path>,
    export_target: Option<&Path>,
    dump_target: Option<&Path>,
) -> Result<ResolvedConfig, CliError> {
    let current_dir = env::current_dir()?;
    let root = repository_root(&current_dir);
    let file = load_file(root.as_deref())?;
    let inputs = Inputs {
        database,
        export_target,
        dump_target,
    };

    Ok(ResolvedConfig {
        database: resolve_path(
            inputs.database,
            "STRATA_DATABASE",
            file.as_ref().and_then(|value| value.database.as_deref()),
            DATABASE_DEFAULT,
            root.as_deref(),
        )?,
        export_target: resolve_path(
            inputs.export_target,
            "STRATA_EXPORT_TARGET",
            file.as_ref()
                .and_then(|value| value.export_target.as_deref()),
            EXPORT_TARGET_DEFAULT,
            root.as_deref(),
        )?,
        dump_target: resolve_path(
            inputs.dump_target,
            "STRATA_DUMP_TARGET",
            file.as_ref().and_then(|value| value.dump_target.as_deref()),
            DUMP_TARGET_DEFAULT,
            root.as_deref(),
        )?,
    })
}

fn resolve_path(
    cli: Option<&Path>,
    environment_name: &str,
    file: Option<&str>,
    default: &str,
    repository_root: Option<&Path>,
) -> Result<ResolvedPath, CliError> {
    if let Some(path) = cli {
        validate_path(path, "CLI flag")?;
        return Ok(ResolvedPath {
            value: path.to_path_buf(),
            source: Source::Cli,
        });
    }

    match env::var(environment_name) {
        Ok(value) => {
            let path = PathBuf::from(value);
            validate_path(&path, environment_name)?;
            return Ok(ResolvedPath {
                value: path,
                source: Source::Env,
            });
        }
        Err(env::VarError::NotPresent) => {}
        Err(env::VarError::NotUnicode(_)) => {
            return Err(CliError::Message(format!(
                "environment variable {environment_name} is not valid UTF-8"
            )))
        }
    }

    if let Some(value) = file {
        let path = PathBuf::from(value);
        validate_path(&path, "configuration file")?;
        let value = match repository_root {
            Some(root) if path.is_relative() => root.join(path),
            _ => path,
        };
        return Ok(ResolvedPath {
            value,
            source: Source::File,
        });
    }

    Ok(ResolvedPath {
        value: PathBuf::from(default),
        source: Source::Default,
    })
}

fn validate_path(path: &Path, source: &str) -> Result<(), CliError> {
    if path.as_os_str().is_empty() {
        return Err(CliError::Message(format!(
            "{source} contains an empty path"
        )));
    }
    Ok(())
}

fn repository_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join(".git").exists() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn load_file(root: Option<&Path>) -> Result<Option<FileConfig>, CliError> {
    let Some(root) = root else {
        return Ok(None);
    };
    let path = root.join(CONFIG_FILE);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    serde_json::from_str(&contents)
        .map(Some)
        .map_err(|error| CliError::Message(format!("failed to parse {}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn environment_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn precedence_is_cli_then_environment_then_file_then_default() {
        let _guard = environment_lock().lock().expect("environment lock");
        let directory = tempfile_directory("precedence");
        let root = directory.join("repo");
        fs::create_dir_all(root.join("nested")).expect("nested directory");
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::write(
            root.join(CONFIG_FILE),
            r#"{"database":"file.db","export_target":"file-export","dump_target":"file.sql"}"#,
        )
        .expect("config");
        let current = env::current_dir().expect("current directory");
        env::set_current_dir(root.join("nested")).expect("nested directory");
        env::set_var("STRATA_DATABASE", "env.db");
        let config = resolve(Some(Path::new("cli.db")), None, None).expect("config");
        assert_eq!(config.database.source, Source::Cli);
        assert_eq!(config.export_target.source, Source::File);
        assert_eq!(config.dump_target.source, Source::File);
        env::remove_var("STRATA_DATABASE");
        env::set_current_dir(current).expect("restore directory");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn missing_file_falls_through_to_defaults() {
        let _guard = environment_lock().lock().expect("environment lock");
        let directory = tempfile_directory("missing");
        let root = directory.join("repo");
        fs::create_dir_all(root.join(".git")).expect("git marker");
        let current = env::current_dir().expect("current directory");
        env::set_current_dir(&root).expect("repository directory");
        env::remove_var("STRATA_DATABASE");
        let config = resolve(None, None, None).expect("config");
        assert_eq!(config.database.source, Source::Default);
        env::set_current_dir(current).expect("restore directory");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn malformed_file_is_an_error() {
        let _guard = environment_lock().lock().expect("environment lock");
        let directory = tempfile_directory("malformed");
        let root = directory.join("repo");
        fs::create_dir_all(root.join(".git")).expect("git marker");
        fs::write(root.join(CONFIG_FILE), "not json").expect("config");
        let current = env::current_dir().expect("current directory");
        env::set_current_dir(&root).expect("repository directory");
        let error = resolve(None, None, None).expect_err("malformed config must fail");
        assert!(error.to_string().contains("failed to parse"));
        env::set_current_dir(current).expect("restore directory");
        fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    fn tempfile_directory(name: &str) -> PathBuf {
        let path =
            env::temp_dir().join(format!("strata-config-test-{name}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temporary directory");
        path
    }
}
