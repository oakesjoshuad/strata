mod args;
#[allow(dead_code)]
mod assets;
mod commands;
mod config;
mod document;
mod dump;
mod export;
mod help;
#[allow(dead_code)]
mod manifest;
mod output;
mod parse;
mod publish;
mod render;
#[allow(dead_code)]
mod render_backend;
#[allow(dead_code)]
mod stage;
mod validate;

use args::{Cli, Command};
use clap::Parser;
use output::output_relationships;
use std::fs;
use store::{Store, StoreError};
use thiserror::Error;

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
    let command = cli.command;
    let config = config::resolve(
        cli.database.as_deref(),
        command.export_target(),
        command.dump_target(),
        cli.publish_target.as_deref(),
        cli.renderer_command.as_deref(),
    )?;
    let path = config.database.value.clone();
    match command {
        Command::Init => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = Store::open(&path)?;
            println!("initialized {}", path.display());
        }
        Command::Schema { .. } | Command::Template { .. } | Command::Capabilities { .. } => {
            commands::execute_without_store(command, &config)?
        }
        Command::Relationships { json: json_flag } => output_relationships(json_flag)?,
        command => {
            let mut store = Store::open(&path)?;
            commands::execute(command, &mut store, &config)?;
        }
    }
    Ok(())
}

impl Command {
    fn export_target(&self) -> Option<&std::path::Path> {
        match self {
            Self::Export { target, .. } => target.as_deref(),
            _ => None,
        }
    }

    fn dump_target(&self) -> Option<&std::path::Path> {
        match self {
            Self::Dump { target, .. } => target.as_deref(),
            _ => None,
        }
    }
}
