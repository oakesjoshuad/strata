use crate::config::ResolvedConfig;
use crate::output::output_capabilities;
use crate::CliError;

pub(crate) fn execute(json: bool, config: &ResolvedConfig) -> Result<(), CliError> {
    output_capabilities(json, config)
}
