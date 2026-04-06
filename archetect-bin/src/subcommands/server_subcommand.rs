use clap::ArgMatches;

use archetect_core::errors::ArchetectError;
use archetect_core::server::{ArchetectServer, ArchetectServiceCore};
use archetect_core::Archetect;

pub fn handle_server_subcommand(
    args: &ArgMatches,
    archetect: Archetect,
) -> Result<(), ArchetectError> {
    let host = args
        .get_one::<String>("host")
        .expect("Has default")
        .to_string();
    let port = *args.get_one::<u16>("port").expect("Has default");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Viable Tokio Runtime");

    runtime
        .block_on(async {
            let core = ArchetectServiceCore::builder(archetect).build().await?;
            let server = ArchetectServer::builder(core)
                .with_host(host)
                .with_port(port)
                .build()
                .await?;

            tokio::select! {
                result = server.serve() => result,
                _ = tokio::signal::ctrl_c() => Ok(()),
            }
        })
        .map_err(|err| ArchetectError::ServerError(err.to_string()))?;

    Ok(())
}
