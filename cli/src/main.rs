use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use strata_records::{RecordId, RecordKind, Status};
use strata_store::{Graph, Store, StoreError};
use thiserror::Error;

const DB_PATH: &str = ".strata/strata.db";

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
#[derive(Parser)]
#[command(name = "strata", version = build_version(), about = "Local-first engineering knowledge records")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, global = true, value_name = "PATH")]
    database: Option<PathBuf>,
}
#[derive(Subcommand)]
enum Command {
    Init,
    New {
        kind: KindArg,
        title: String,
        #[arg(long)]
        document: Option<String>,
    },
    Show {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Search {
        query: String,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    Graph {
        id: String,
        #[arg(long)]
        json: bool,
    },
    History {
        id: String,
    },
    Link {
        source: String,
        relation: String,
        target: String,
    },
    Status {
        id: String,
        new_status: String,
    },
    Revise {
        id: String,
        #[arg(long)]
        document: String,
        #[arg(long)]
        summary: Option<String>,
    },
    Validate {
        #[arg(long)]
        json: bool,
    },
    Schema {
        kind: KindArg,
        #[arg(long)]
        json: bool,
    },
    Capabilities {
        #[arg(long)]
        json: bool,
    },
    Relationships {
        #[arg(long)]
        json: bool,
    },
}
#[derive(Clone, ValueEnum)]
enum KindArg {
    Rfc,
    Pdr,
    Adr,
    Edr,
}
impl From<KindArg> for RecordKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Rfc => Self::Rfc,
            KindArg::Pdr => Self::Pdr,
            KindArg::Adr => Self::Adr,
            KindArg::Edr => Self::Edr,
        }
    }
}

fn build_version() -> &'static str {
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
        } => {
            let kind: RecordKind = kind.into();
            let document = match document {
                Some(s) => serde_json::from_str(&s)?,
                None => kind.default_document(&title),
            };
            print_json_or_text(store.create(kind, &title, document)?)?;
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
                println!("{}", serde_json::to_string_pretty(&records)?);
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
        Command::Status { id, new_status } => {
            let status = Status::from_str(&new_status).map_err(CliError::Message)?;
            print_json_or_text(store.set_status(&parse_id(&id)?, status)?)?;
        }
        Command::Revise {
            id,
            document,
            summary,
        } => {
            print_json_or_text(store.revise(
                &parse_id(&id)?,
                serde_json::from_str(&document)?,
                summary.as_deref(),
            )?)?;
        }
        Command::Validate { json: json_flag } => {
            let issues = store.validate()?;
            if json_flag {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &json!({"valid": issues.is_empty(), "issues": issues})
                    )?
                );
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
fn print_json_or_text<T: serde::Serialize>(value: T) -> Result<(), CliError> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
fn print_value<T: serde::Serialize + std::fmt::Debug>(
    value: T,
    json_flag: bool,
) -> Result<(), CliError> {
    if json_flag {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{:?}", value);
    }
    Ok(())
}
fn print_graph(graph: Graph, json_flag: bool) -> Result<(), CliError> {
    if json_flag {
        println!("{}", serde_json::to_string_pretty(&graph)?);
    } else {
        println!(
            "{} [{}] {}",
            graph.record.id, graph.record.status, graph.record.title
        );
        for relation in graph.relationships {
            println!(
                "  {} {} {}",
                relation.source_id, relation.relation, relation.target_id
            );
        }
    }
    Ok(())
}
fn output_schema(kind: RecordKind, json_flag: bool) -> Result<(), CliError> {
    let value = json!({"kind": kind, "schema": format!("{}/v1", kind.slug()), "required_fields": kind.required_fields(), "statuses": kind.statuses()});
    if json_flag {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{}", serde_json::to_string(&value)?);
    }
    Ok(())
}
fn output_capabilities(json_flag: bool) -> Result<(), CliError> {
    let value = json!({"version": build_version(), "kinds": RecordKind::ALL, "commands": ["init", "new", "show", "search", "graph", "history", "link", "status", "revise", "validate", "schema", "capabilities", "relationships"], "json_output": ["show", "search", "graph", "validate", "schema", "capabilities", "relationships"]});
    if json_flag {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{}", serde_json::to_string(&value)?);
    }
    Ok(())
}
fn output_relationships(json_flag: bool) -> Result<(), CliError> {
    let value = json!({"relationships": strata_records::RELATIONSHIPS});
    if json_flag {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{}", serde_json::to_string(&value)?);
    }
    Ok(())
}
