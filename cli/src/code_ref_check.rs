use crate::config::ResolvedConfig;
use crate::output::emit_json;
use crate::symbol_locator::{self, LocatorError};
use crate::CliError;
use records::CodeReference;
use serde::Serialize;
use std::path::Path;
use std::str::FromStr;
use store::Store;

#[derive(Debug, Serialize)]
pub(crate) struct CheckResult {
    pub(crate) status: &'static str,
    pub(crate) record_id: String,
    pub(crate) path: String,
    pub(crate) symbol: Option<String>,
}

pub(crate) fn run(
    store: &Store,
    config: &ResolvedConfig,
    id: Option<&str>,
    code_root: &Path,
    json: bool,
) -> Result<(), CliError> {
    let references = store.list_all_code_references()?;
    let selected = id
        .map(|value| records::RecordId::from_str(value).map_err(CliError::Message))
        .transpose()?;
    let results = references
        .into_iter()
        .filter(|reference| {
            selected
                .as_ref()
                .is_none_or(|id| id == &reference.record_id)
        })
        .map(|reference| {
            check_one(
                &reference,
                config.code_ref_locator.value.as_str(),
                code_root,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let valid = results.iter().all(|result| result.status == "OK");
    if json {
        emit_json(serde_json::json!({"valid": valid, "issues": results}), true)?;
    } else {
        for result in &results {
            println!(
                "{} {} {} {}",
                result.status,
                result.record_id,
                result.path,
                result.symbol.as_deref().map_or("-", |symbol| symbol)
            );
        }
    }
    if valid {
        Ok(())
    } else {
        Err(CliError::Message("code reference check failed".into()))
    }
}

fn check_one(
    reference: &CodeReference,
    locator: &str,
    code_root: &Path,
) -> Result<CheckResult, CliError> {
    let status = if !code_root.join(&reference.path).is_file() {
        "FILE-MISSING"
    } else if let Some(symbol) = reference.symbol.as_deref() {
        match symbol_locator::locate(locator, code_root, symbol, Path::new(&reference.path)) {
            Ok(location)
                if reference.line_start == Some(location.line_start)
                    && reference.line_end == Some(location.line_end) =>
            {
                "OK"
            }
            Ok(_) => "LINE-DRIFT",
            Err(LocatorError::Missing) => "SYMBOL-MISSING",
            Err(LocatorError::Ambiguous) => "SYMBOL-AMBIGUOUS",
            Err(LocatorError::Unavailable(_)) => "LOCATOR-UNAVAILABLE",
        }
    } else {
        "OK"
    };
    Ok(CheckResult {
        status,
        record_id: reference.record_id.to_string(),
        path: reference.path.clone(),
        symbol: reference.symbol.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ResolvedPath, ResolvedValue, Source};
    use records::RecordKind;
    use std::path::PathBuf;

    fn config(locator: String) -> ResolvedConfig {
        ResolvedConfig {
            database: ResolvedPath {
                value: PathBuf::from("db"),
                source: Source::Default,
            },
            export_target: ResolvedPath {
                value: PathBuf::from("export"),
                source: Source::Default,
            },
            dump_target: ResolvedPath {
                value: PathBuf::from("dump"),
                source: Source::Default,
            },
            publish_target: ResolvedPath {
                value: PathBuf::from("site"),
                source: Source::Default,
            },
            publish_renderer: ResolvedValue {
                value: "renderer".into(),
                source: Source::Default,
            },
            code_ref_locator: ResolvedValue {
                value: locator,
                source: Source::Default,
            },
        }
    }

    #[test]
    fn pipeline_reports_missing_file_without_invoking_locator() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("record");
        store
            .add_code_reference(
                &record.id,
                "constrains",
                "missing.rs",
                Some("target"),
                Some(1),
                Some(1),
            )
            .expect("reference");
        let root =
            std::env::temp_dir().join(format!("strata-code-ref-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("root");
        let command = "sh -c 'exit 99'".to_owned();
        let error = run(&store, &config(command), None, &root, true).expect_err("failure");
        assert!(error.to_string().contains("code reference check failed"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn pipeline_accepts_reference_verified_by_fake_locator() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(RecordKind::Adr, "x", RecordKind::Adr.default_document("x"))
            .expect("record");
        let root =
            std::env::temp_dir().join(format!("strata-code-ref-success-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).expect("directory");
        std::fs::write(root.join("src/lib.rs"), "fn target() {}\n").expect("file");
        store
            .add_code_reference(
                &record.id,
                "constrains",
                "src/lib.rs",
                Some("target"),
                Some(1),
                Some(1),
            )
            .expect("reference");
        let script = r#"if test "$1" = resolve; then printf '<resolution query="target" candidates="1" selected_id="1"><symbol stable_id="src/lib.rs::fn::target" file="./src/lib.rs"/></resolution>'; else printf '<graph><symbol stable_id="src/lib.rs::fn::other" range="L20-L30"/><symbol stable_id="src/lib.rs::fn::target" range="L1-L1"/></graph>'; fi"#;
        let command = format!("sh -c {} sh", shell_words::quote(script));
        run(&store, &config(command), None, &root, true).expect("check");
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
