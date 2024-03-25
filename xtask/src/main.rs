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

fn build(_args: &ArgMatches) -> anyhow::Result<()> {
    let cargo = std::env::var("CARGO")?;
    std::process::Command::new(cargo)
        .env("OPENSSL_STATIC", "1")
        .args(["install", "--path=archetect-bin"])
        .status()
        .expect("Error installing Archetect");
    Ok(())
}
