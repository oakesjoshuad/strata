mod args;
mod output;

use args::{Cli, Command};
use clap::Parser;
use output::{
    emit_json, output_capabilities, output_relationships, output_schema, print_graph, print_value,
};
use records::{RecordId, RecordKind, Status};
use serde_json::json;
use std::fs;
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
            json: json_flag,
        } => {
            let kind: RecordKind = kind.into();
            let document = match document {
                Some(s) => serde_json::from_str(&s)?,
                None => kind.default_document(&title),
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
        Command::Status {
            id,
            new_status,
            json: json_flag,
        } => {
            let status = Status::from_str(&new_status).map_err(CliError::Message)?;
            print_value(store.set_status(&parse_id(&id)?, status)?, json_flag)?;
        }
        Command::Revise {
            id,
            document,
            summary,
            json: json_flag,
        } => {
            print_value(
                store.revise(
                    &parse_id(&id)?,
                    serde_json::from_str(&document)?,
                    summary.as_deref(),
                )?,
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
        | Command::Capabilities { .. }
        | Command::Relationships { .. } => unreachable!(),
    }
    Ok(())
}

fn parse_id(value: &str) -> Result<RecordId, CliError> {
    RecordId::from_str(value).map_err(CliError::Message)
}
