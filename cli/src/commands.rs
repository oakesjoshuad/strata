mod auxiliary;
mod capabilities;
mod context;
mod schema;

use crate::args::Command;
use crate::config;
use crate::CliError;
use store::Store;

pub(crate) fn execute(
    command: Command,
    store: &mut Store,
    config: &config::ResolvedConfig,
) -> Result<(), CliError> {
    match command {
        Command::Capabilities { json } => capabilities::execute(json, config),
        Command::Context {
            query,
            json,
            limit,
            offset,
        } => context::execute(store, query, json, limit, offset),
        Command::Schema { kind, json } => schema::execute_schema(kind.into(), json),
        Command::Template { kind } => schema::execute_template(kind.into()),
        command => auxiliary::execute(command, store, config),
    }
}

pub(crate) fn execute_without_store(
    command: Command,
    config: &config::ResolvedConfig,
) -> Result<(), CliError> {
    match command {
        Command::Capabilities { json } => capabilities::execute(json, config),
        Command::Schema { kind, json } => schema::execute_schema(kind.into(), json),
        Command::Template { kind } => schema::execute_template(kind.into()),
        _ => unreachable!(),
    }
}
