use clap::ArgMatches;
use archetect_core::Archetect;

pub fn handle_commands_subcommand(_args: &ArgMatches, archetect: &Archetect) {
    let mut keys = vec![];
    for key in archetect.configuration().commands().keys() {
        keys.push(key);
    }
    keys.sort();
    for key in keys {
        println!("{}", key);
    }
}