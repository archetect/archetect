use clap::ArgMatches;

use archetect_grpc::{ArchetectServer, ArchetectServiceCore};

pub fn handle_server_subcommand(args: &ArgMatches) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::
    new_multi_thread()
        .build()
        .expect("Viable Tokio Runtime")
        ;

    runtime.block_on(async {
        let core = ArchetectServiceCore::builder().build().await?;
        let server = ArchetectServer::builder(core)
            .build().await?;

        tokio::select! {
                result = server.serve() => {
                  return result;
                },
                _ = tokio::signal::ctrl_c() => {
                    return Ok(());
                },
            }
    })?;

    Ok(())
}
