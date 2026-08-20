use crate::output::print_context;
use crate::CliError;
use store::Store;

pub(crate) fn execute(
    store: &mut Store,
    query: String,
    json: bool,
    limit: u32,
    offset: u32,
) -> Result<(), CliError> {
    let records = store.search(&query, limit, offset)?;
    let graphs = records
        .iter()
        .map(|record| store.graph(&record.id))
        .collect::<Result<Vec<_>, _>>()?;
    print_context(&query, graphs, json)
}
