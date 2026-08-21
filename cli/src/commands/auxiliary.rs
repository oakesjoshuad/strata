use crate::args::Command;
use crate::code_ref_check;
use crate::config;
use crate::output::{
    emit_json, print_code_reference, print_dump_result, print_evidence, print_graph,
    print_relationships, print_restored, print_value,
};
use crate::render::render_record;
use crate::{dump, export, parse, publish, validate, CliError};
use records::{RecordId, RecordKind, Status};
use serde_json::json;
use std::io::{self, IsTerminal};
use std::str::FromStr;
use store::Store;

pub(crate) fn execute(
    command: Command,
    store: &mut Store,
    config: &config::ResolvedConfig,
) -> Result<(), CliError> {
    match command {
        Command::New {
            kind,
            title,
            document,
            file,
            json: json_flag,
        } => {
            let kind: RecordKind = kind.into();
            let document = match (document, file) {
                (Some(_), Some(_)) => {
                    return Err(CliError::Message(
                        "new accepts only one of --document or --file".into(),
                    ))
                }
                (Some(s), None) => serde_json::from_str(&s)?,
                (None, Some(path)) => parse::read_file(&path, kind)?,
                (None, None) if !io::stdin().is_terminal() => parse::read_stdin(kind)?,
                (None, None) => kind.default_document(&title),
            };
            print_value(store.create(kind, &title, document)?, json_flag)?;
        }
        Command::Show {
            id,
            json: json_flag,
        } => {
            let record = store.get(&parse_id(&id)?);
            print_value(record?, json_flag)?;
        }
        Command::Search {
            query,
            json: json_flag,
            limit,
            offset,
        } => {
            let records = store.search(&query, limit, offset)?;
            if json_flag {
                emit_json(records, true)?;
            } else {
                for record in records {
                    println!("{} [{}] {}", record.id, record.status, record.title);
                }
            }
        }
        Command::Graph {
            id,
            json: json_flag,
        } => {
            let graph = store.graph(&parse_id(&id)?)?;
            print_graph(graph, json_flag)?;
        }
        Command::Render { id } => {
            print!("{}", render_record(store, &parse_id(&id)?)?);
        }
        Command::Export { check, .. } => export::run(store, check, &config.export_target.value)?,
        Command::Dump { check, .. } => print_dump_result(
            dump::run(store, check, &config.dump_target.value)?,
            &config.dump_target.value,
        )?,
        Command::Publish { check, json } => {
            let outcome = publish::run(store, check, config)?;
            if json {
                emit_json(&outcome, true)?;
            } else if check {
                if outcome.clean {
                    println!("publish check clean");
                } else {
                    for issue in &outcome.issues {
                        println!("{issue}");
                    }
                    return Err(CliError::Message("publish check failed".into()));
                }
            } else {
                println!("published {} records", outcome.published);
            }
        }
        Command::Restore { path } => {
            dump::restore(store, &path)?;
            print_restored(&path);
        }
        Command::History { id } => {
            for revision in store.history(&parse_id(&id)?)? {
                println!(
                    "r{} {} {}",
                    revision.revision,
                    revision.changed_at,
                    revision.change_summary.unwrap_or_else(|| "changed".into())
                );
            }
        }
        Command::Link {
            source,
            relation,
            targets,
            json: json_flag,
        } => {
            let targets = targets
                .iter()
                .map(|target| parse_id(target))
                .collect::<Result<Vec<_>, _>>()?;
            print_relationships(
                store.link_many(&parse_id(&source)?, &relation, &targets)?,
                json_flag,
            )?;
        }
        Command::EvidenceAdd {
            id,
            kind,
            title,
            uri,
            content,
            metadata,
            json: json_flag,
        } => {
            let metadata = metadata.as_deref().map(serde_json::from_str).transpose()?;
            let evidence = store.add_evidence(
                &parse_id(&id)?,
                &kind,
                &title,
                uri.as_deref(),
                content.as_deref(),
                metadata.as_ref(),
            )?;
            print_evidence(evidence, json_flag)?;
        }
        Command::CodeRefAdd {
            id,
            relation,
            path,
            symbol,
            line_start,
            line_end,
            json: json_flag,
        } => {
            let code_reference = store.add_code_reference(
                &parse_id(&id)?,
                &relation,
                &path,
                symbol.as_deref(),
                line_start,
                line_end,
            )?;
            print_code_reference(code_reference, json_flag)?;
        }
        Command::Status {
            id,
            new_status,
            json: json_flag,
        } => {
            let status = Status::from_str(&new_status).map_err(CliError::Message)?;
            print_value(store.set_status(&parse_id(&id)?, status)?, json_flag)?;
        }
        Command::Retitle {
            id,
            title,
            json: json_flag,
        } => {
            print_value(store.retitle(&parse_id(&id)?, &title)?, json_flag)?;
        }
        Command::Revise {
            id,
            document,
            file,
            summary,
            json: json_flag,
        } => {
            if document.is_some() && file.is_some() {
                return Err(CliError::Message(
                    "revise accepts only one of --document or --file".into(),
                ));
            }
            let record_id = parse_id(&id)?;
            let document = match (document, file) {
                (Some(document), None) => serde_json::from_str(&document)?,
                (None, Some(path)) => parse::read_file_for_id(&path, &record_id)?,
                (None, None) if !io::stdin().is_terminal() => parse::read_stdin_for_id(&record_id)?,
                (None, None) => {
                    return Err(CliError::Message(
                        "revise requires --document, --file, or Markdown on stdin".into(),
                    ))
                }
                (Some(_), Some(_)) => unreachable!(),
            };
            print_value(
                store.revise(&record_id, document, summary.as_deref())?,
                json_flag,
            )?;
        }
        Command::Validate { json: json_flag } => {
            let issues = validate::issues(store, config)?;
            let has_errors = issues.iter().any(|issue| !issue.starts_with("WARN "));
            if json_flag {
                emit_json(json!({"valid": !has_errors, "issues": issues}), true)?;
            } else if issues.is_empty() {
                println!("valid");
            } else {
                for issue in issues {
                    if issue.starts_with("WARN ") {
                        println!("{issue}");
                    } else {
                        println!("ERROR {issue}");
                    }
                }
                if !has_errors {
                    return Ok(());
                }
                return Err(CliError::Message("database validation failed".into()));
            }
        }
        Command::CodeRefCheck {
            id,
            code_root,
            json,
        } => {
            code_ref_check::run(store, config, id.as_deref(), &code_root, json)?;
        }
        Command::Init
        | Command::Schema { .. }
        | Command::Template { .. }
        | Command::Capabilities { .. }
        | Command::Context { .. }
        | Command::Relationships { .. } => unreachable!(),
    }
    Ok(())
}

fn parse_id(value: &str) -> Result<RecordId, CliError> {
    RecordId::from_str(value).map_err(CliError::Message)
}
