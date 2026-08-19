mod args;
mod document;
mod export;
mod help;
mod output;
mod parse;
mod render;

use args::{Cli, Command};
use clap::Parser;
use output::{
    emit_json, output_capabilities, output_relationships, output_schema, print_code_reference,
    print_evidence, print_graph, print_value,
};
use records::{RecordId, RecordKind, Status};
use render::{render_record, render_template};
use serde_json::json;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::str::FromStr;
use store::{Store, StoreError};
use thiserror::Error;

const DB_PATH: &str = ".strata/strata.db";

#[derive(Debug, Error)]
pub(crate) enum CliError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub(crate) fn build_version() -> &'static str {
    env!("STRATA_VERSION")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if help::requested(&args) {
        help::print();
        return;
    }
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let path = cli.database.unwrap_or_else(|| PathBuf::from(DB_PATH));
    match cli.command {
        Command::Init => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = Store::open(&path)?;
            println!("initialized {}", path.display());
        }
        Command::Schema {
            kind,
            json: json_flag,
        } => output_schema(kind.into(), json_flag)?,
        Command::Template { kind } => print!("{}", render_template(kind.into())?),
        Command::Capabilities { json: json_flag } => output_capabilities(json_flag)?,
        Command::Relationships { json: json_flag } => output_relationships(json_flag)?,
        command => {
            let mut store = Store::open(&path)?;
            execute(command, &mut store)?;
        }
    }
    Ok(())
}

fn execute(command: Command, store: &mut Store) -> Result<(), CliError> {
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
        Command::Export { check } => export::run(store, check)?,
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
            target,
        } => {
            let edge = store.link(&parse_id(&source)?, &relation, &parse_id(&target)?)?;
            println!("{} {} {}", edge.source_id, edge.relation, edge.target_id);
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
            let issues = store.validate()?;
            if json_flag {
                emit_json(json!({"valid": issues.is_empty(), "issues": issues}), true)?;
            } else if issues.is_empty() {
                println!("valid");
            } else {
                for issue in issues {
                    println!("ERROR {issue}");
                }
                return Err(CliError::Message("database validation failed".into()));
            }
        }
        Command::Init
        | Command::Schema { .. }
        | Command::Template { .. }
        | Command::Capabilities { .. }
        | Command::Relationships { .. } => unreachable!(),
    }
    Ok(())
}

fn parse_id(value: &str) -> Result<RecordId, CliError> {
    RecordId::from_str(value).map_err(CliError::Message)
}
