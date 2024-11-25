use clap::ArgMatches;
use archetect_core::Archetect;
use archetect_core::errors::ArchetectError;

pub fn handle_check_subcommand(_matches: &ArgMatches, archetect: &Archetect) -> Result<(), ArchetectError> {
    archetect.check()
}