use crate::{config::ResolvedConfig, dump, export, CliError};
use store::Store;

pub(crate) fn issues(store: &Store, config: &ResolvedConfig) -> Result<Vec<String>, CliError> {
    let mut issues = store.validate()?;
    issues.extend(export::stale_messages(store, &config.export_target.value)?);
    issues.extend(dump::stale_messages(store, &config.dump_target.value)?);
    Ok(issues)
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
                value: PathBuf::from("/tmp/strata-export-missing"),
                source: Source::Default,
            },
            dump_target: ResolvedPath {
                value: PathBuf::from("/tmp/strata-dump-missing.sql"),
                source: Source::Default,
            },
            publish_target: ResolvedPath {
                value: PathBuf::from("/tmp/strata-site"),
                source: Source::Default,
            },
            publish_renderer: ResolvedValue {
                value: "renderer".into(),
                source: Source::Default,
            },
            code_ref_locator: ResolvedValue {
                value: "graphlite".into(),
                source: Source::Default,
            },
        }
    }

    #[test]
    fn returns_store_validation_issues() {
        let mut store = Store::open_memory().expect("store");
        let record = store
            .create(
                RecordKind::Adr,
                "Accepted",
                RecordKind::Adr.default_document("Accepted"),
            )
            .expect("record");
        store
            .set_status(&record.id, records::Status::Proposed)
            .expect("propose");
        store
            .set_status(&record.id, records::Status::Accepted)
            .expect("accept");
        let issues = issues(&store, &config()).expect("issues");
        assert!(issues.iter().any(|issue| issue.starts_with("WARN ")));
    }
}
