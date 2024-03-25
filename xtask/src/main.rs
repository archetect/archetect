use std::process::Command;
use clap::ArgMatches;

mod cli;

fn main() -> anyhow::Result<()> {
    let args = cli::command()
        .get_matches();

    match args.subcommand() {
        None => println!("Subcommands are required, and should be handled by Clap configuration"),
        Some(("install", args)) => {
            build(args)?;
        }
        Some((command, _)) => {
            unimplemented!("{} command not implemented, and should be checked by Clap", command)
        }
    }

    Ok(())
}

fn build(args: &ArgMatches) -> anyhow::Result<()> {
    let cargo = std::env::var("CARGO")?;
    let mut command = Command::new(cargo);
    command
        .arg("install")
        .arg("--path=archetect-bin")
        ;

    if args.get_flag("openssl-static") {
        command.env("OPENSSL_STATIC", "1");
    }

    command
        .status()
        .expect("Error installing Archetect");

    Ok(())
}
