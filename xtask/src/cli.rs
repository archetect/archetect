use clap::Command;

pub fn command() -> Command {
    Command::new("xtask")
        .help_expected(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("install")
                .about("Install Archetect")
        )
}