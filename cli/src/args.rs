use clap::{Parser, Subcommand, ValueEnum};
use records::RecordKind;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "strata", version = crate::build_version(), about = "Local-first engineering knowledge records")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
    #[arg(long, global = true, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    Init,
    New {
        kind: KindArg,
        title: String,
        #[arg(long)]
        document: Option<String>,
        #[arg(long)]
        json: bool,
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
        #[arg(long)]
        json: bool,
    },
    Revise {
        id: String,
        #[arg(long)]
        document: String,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        json: bool,
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
pub(crate) enum KindArg {
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
