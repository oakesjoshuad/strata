use clap::{Parser, Subcommand, ValueEnum};
use records::RecordKind;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "strata", version = crate::build_version(), about = "Local-first engineering knowledge records")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
    /// Path to the SQLite database (CLI, STRATA_DATABASE, config file, or default)
    #[arg(long, global = true, value_name = "PATH")]
    pub(crate) database: Option<PathBuf>,
    /// Publication artifact directory
    #[arg(long, global = true, value_name = "PATH")]
    pub(crate) publish_target: Option<PathBuf>,
    /// External publication renderer command template
    #[arg(long, global = true, value_name = "COMMAND")]
    pub(crate) renderer_command: Option<String>,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Initialize a new database
    Init,
    /// Create a new RFC, PDR, ADR, or EDR record
    New {
        /// Record kind
        kind: KindArg,
        /// Record title
        title: String,
        /// Document content as a JSON object
        #[arg(long)]
        document: Option<String>,
        /// Read document content from a Markdown file
        #[arg(long)]
        file: Option<PathBuf>,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show a record's current content
    Show {
        /// Record id, e.g. ADR-0001
        id: String,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Full-text search across records
    Search {
        /// Search query
        query: String,
        /// Print the results as JSON
        #[arg(long)]
        json: bool,
        /// Maximum number of results
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Number of results to skip
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Retrieve bounded search results with each record's one-hop graph context
    Context {
        /// Full-text query
        query: String,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
        /// Maximum number of records to expand
        #[arg(long, default_value_t = 10)]
        limit: u32,
        /// Number of matching records to skip
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Show a record's relationships, evidence, and code references
    Graph {
        /// Record id, e.g. ADR-0001
        id: String,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Render a record to Markdown
    Render {
        /// Record id, e.g. ADR-0001
        id: String,
    },
    /// Export all records to Markdown, or verify existing exports with --check
    Export {
        /// Verify committed Markdown matches canonical state instead of writing it
        #[arg(long)]
        check: bool,
        /// Target directory for generated Markdown
        #[arg(long, value_name = "PATH")]
        target: Option<PathBuf>,
    },
    /// Dump the full-fidelity database snapshot to the configured target
    Dump {
        /// Verify the committed SQL snapshot matches canonical state instead of writing it
        #[arg(long)]
        check: bool,
        /// Target path for the native SQL snapshot
        #[arg(long, value_name = "PATH")]
        target: Option<PathBuf>,
    },
    /// Publish all records as a static site, or verify the publication manifest with --check
    Publish {
        /// Verify the publication manifest instead of rendering and writing the site
        #[arg(long)]
        check: bool,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Restore a database from a native SQL dump into the --database target
    Restore {
        /// Path to the native SQL dump
        path: PathBuf,
    },
    /// Show a record's revision history
    History {
        /// Record id, e.g. ADR-0001
        id: String,
    },
    /// Create a relationship between two records
    Link {
        /// Source record id
        source: String,
        /// Relationship kind; see `strata relationships` for the full valid set
        relation: String,
        /// Target record ids
        #[arg(required = true, num_args = 1..)]
        targets: Vec<String>,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Attach supporting evidence to a record
    EvidenceAdd {
        /// Record id the evidence supports
        id: String,
        /// Evidence kind; see `strata relationships` for the full valid set
        kind: String,
        /// Evidence title
        title: String,
        /// Link to the evidence source
        #[arg(long)]
        uri: Option<String>,
        /// Inline evidence content
        #[arg(long)]
        content: Option<String>,
        /// Structured evidence metadata as a JSON object
        #[arg(long)]
        metadata: Option<String>,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Attach a code reference to a record
    CodeRefAdd {
        /// Record id the code reference supports
        id: String,
        /// Relationship kind, e.g. constrains, implements
        relation: String,
        /// Path to the referenced file
        path: String,
        /// Referenced symbol name, e.g. Store::revise
        #[arg(long)]
        symbol: Option<String>,
        /// First referenced line
        #[arg(long)]
        line_start: Option<u32>,
        /// Last referenced line
        #[arg(long)]
        line_end: Option<u32>,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Transition a record to a new lifecycle status
    Status {
        /// Record id, e.g. ADR-0001
        id: String,
        /// New status, e.g. accepted, superseded
        new_status: String,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Change a record's title
    Retitle {
        /// Record id, e.g. ADR-0001
        id: String,
        /// New title
        title: String,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Update a record's document content
    Revise {
        /// Record id, e.g. ADR-0001
        id: String,
        /// Document content as a JSON object
        #[arg(long)]
        document: Option<String>,
        /// Read document content from a Markdown file
        #[arg(long)]
        file: Option<PathBuf>,
        /// One-line summary of the change
        #[arg(long)]
        summary: Option<String>,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Check the database for structural and lifecycle issues
    Validate {
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show a record kind's required document fields
    Schema {
        /// Record kind
        kind: KindArg,
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Print a blank Markdown template for a record kind
    Template {
        /// Record kind
        kind: KindArg,
    },
    /// Show CLI version and supported operations, for self-describing agents
    Capabilities {
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
    /// List valid relationship and evidence kinds
    Relationships {
        /// Print the result as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, ValueEnum)]
pub(crate) enum KindArg {
    /// Should this problem or proposal be pursued?
    Rfc,
    /// What does the proposed design look like and what evidence supports it?
    Pdr,
    /// What architecturally significant choice was made and why?
    Adr,
    /// What implementation-level engineering choice was made?
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
