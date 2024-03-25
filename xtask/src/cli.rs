use clap::{Arg, Command};
use clap::builder::BoolishValueParser;

pub fn command() -> Command {
    Command::new("xtask")
        .help_expected(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("install")
                .about("Install Archetect")
                .arg(
                    Arg::new("openssl-static")
                        .help("Whether or not to statically compile OpenSSL into the resulting application")
                        .long("static-openssl")
                        .visible_alias("static-ssl")
                        .default_missing_value("true")
                        .default_value("true")
                        .num_args(0..=1)
                        .value_parser(BoolishValueParser::new())
                )
        )
}