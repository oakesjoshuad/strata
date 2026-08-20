use crate::output::output_schema;
use crate::render::render_template;
use crate::CliError;
use records::RecordKind;

pub(crate) fn execute_schema(kind: RecordKind, json: bool) -> Result<(), CliError> {
    output_schema(kind, json)
}

pub(crate) fn execute_template(kind: RecordKind) -> Result<(), CliError> {
    print!("{}", render_template(kind)?);
    Ok(())
}
