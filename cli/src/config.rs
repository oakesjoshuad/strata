use crate::CliError;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const DATABASE_DEFAULT: &str = ".strata/strata.db";
pub(crate) const EXPORT_TARGET_DEFAULT: &str = "docs/records";
pub(crate) const DUMP_TARGET_DEFAULT: &str = "docs/db-snapshot.sql";
pub(crate) const PUBLISH_TARGET_DEFAULT: &str = ".strata/site";
pub(crate) const PUBLISH_RENDERER_DEFAULT: &str =
    "pandoc {input} -o {output} --standalone --metadata-file {metadata} --template {template} --css {css} --lua-filter {lua_filter}";
pub(crate) const CODE_REF_LOCATOR_DEFAULT: &str = "graphlite";
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
#[allow(dead_code)]
pub(crate) struct ResolvedValue {
    pub(crate) value: String,
    pub(crate) source: Source,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ResolvedConfig {
    pub(crate) database: ResolvedPath,
    pub(crate) export_target: ResolvedPath,
    pub(crate) dump_target: ResolvedPath,
    pub(crate) publish_target: ResolvedPath,
    pub(crate) publish_renderer: ResolvedValue,
    pub(crate) code_ref_locator: ResolvedValue,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    database: Option<String>,
    export_target: Option<String>,
    dump_target: Option<String>,
    publish_target: Option<String>,
    publish_renderer: Option<String>,
    code_ref_locator: Option<String>,
}

struct Inputs<'a> {
    database: Option<&'a Path>,
    export_target: Option<&'a Path>,
    dump_target: Option<&'a Path>,
    publish_target: Option<&'a Path>,
    publish_renderer: Option<&'a str>,
    code_ref_locator: Option<&'a str>,
}

pub(crate) fn resolve(
    database: Option<&Path>,
    export_target: Option<&Path>,
    dump_target: Option<&Path>,
    publish_target: Option<&Path>,
    publish_renderer: Option<&str>,
    code_ref_locator: Option<&str>,
) -> Result<ResolvedConfig, CliError> {
    let current_dir = env::current_dir()?;
    let root = repository_root(&current_dir);
    let file = load_file(root.as_deref())?;
    let inputs = Inputs {
        database,
        export_target,
        dump_target,
        publish_target,
        publish_renderer,
        code_ref_locator,
    };
    let database = resolve_path(
        inputs.database,
        "STRATA_DATABASE",
        file.as_ref().and_then(|value| value.database.as_deref()),
        DATABASE_DEFAULT,
        root.as_deref(),
        root.as_deref(),
    )?;
    let projection_root = match &database.source {
        Source::Cli | Source::Env => {
            database_repository_root(&database.value, &current_dir, root.as_deref())
        }
        Source::File | Source::Default => root.clone(),
    };

    Ok(ResolvedConfig {
        database,
        export_target: resolve_path(
            inputs.export_target,
            "STRATA_EXPORT_TARGET",
            file.as_ref()
                .and_then(|value| value.export_target.as_deref()),
            EXPORT_TARGET_DEFAULT,
            root.as_deref(),
            projection_root.as_deref(),
        )?,
        dump_target: resolve_path(
            inputs.dump_target,
            "STRATA_DUMP_TARGET",
            file.as_ref().and_then(|value| value.dump_target.as_deref()),
            DUMP_TARGET_DEFAULT,
            root.as_deref(),
            projection_root.as_deref(),
        )?,
        publish_target: resolve_path(
            inputs.publish_target,
            "STRATA_PUBLISH_TARGET",
            file.as_ref()
                .and_then(|value| value.publish_target.as_deref()),
            PUBLISH_TARGET_DEFAULT,
            root.as_deref(),
            projection_root.as_deref(),
        )?,
        publish_renderer: resolve_value(
            inputs.publish_renderer,
            "STRATA_PUBLISH_RENDERER",
            file.as_ref()
                .and_then(|value| value.publish_renderer.as_deref()),
            PUBLISH_RENDERER_DEFAULT,
        )?,
        code_ref_locator: resolve_value(
            inputs.code_ref_locator,
            "STRATA_CODE_REF_LOCATOR",
            file.as_ref()
                .and_then(|value| value.code_ref_locator.as_deref()),
            CODE_REF_LOCATOR_DEFAULT,
        )?,
    })
}

fn resolve_value(
    cli: Option<&str>,
    environment_name: &str,
    file: Option<&str>,
    default: &str,
) -> Result<ResolvedValue, CliError> {
    if let Some(value) = cli {
        validate_value(value, "CLI flag")?;
        return Ok(ResolvedValue {
            value: value.to_owned(),
            source: Source::Cli,
        });
    }

    match env::var(environment_name) {
        Ok(value) => {
            validate_value(&value, environment_name)?;
            return Ok(ResolvedValue {
                value,
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
        validate_value(value, "configuration file")?;
        return Ok(ResolvedValue {
            value: value.to_owned(),
            source: Source::File,
        });
    }

    Ok(ResolvedValue {
        value: default.to_owned(),
        source: Source::Default,
    })
}

fn resolve_path(
    cli: Option<&Path>,
    environment_name: &str,
    file: Option<&str>,
    default: &str,
    file_root: Option<&Path>,
    default_root: Option<&Path>,
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
        let value = match file_root {
            Some(root) if path.is_relative() => root.join(path),
            _ => path,
        };
        return Ok(ResolvedPath {
            value,
            source: Source::File,
        });
    }

    let path = PathBuf::from(default);
    let value = match default_root {
        Some(root) if path.is_relative() => root.join(path),
        _ => path,
    };
    Ok(ResolvedPath {
        value,
        source: Source::Default,
    })
}

fn database_repository_root(
    database: &Path,
    current_dir: &Path,
    fallback: Option<&Path>,
) -> Option<PathBuf> {
    let absolute = if database.is_relative() {
        current_dir.join(database)
    } else {
        database.to_path_buf()
    };
    let parent = absolute.parent()?;
    match repository_root(parent) {
        Some(root) => Some(root),
        None => fallback.map(Path::to_path_buf),
    }
}

fn validate_path(path: &Path, source: &str) -> Result<(), CliError> {
    if path.as_os_str().is_empty() {
        return Err(CliError::Message(format!(
            "{source} contains an empty path"
        )));
    }
    Ok(())
}

fn validate_value(value: &str, source: &str) -> Result<(), CliError> {
    if value.is_empty() {
        return Err(CliError::Message(format!(
            "{source} contains an empty value"
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
mod tests;
