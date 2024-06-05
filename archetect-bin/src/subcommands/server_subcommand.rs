use archetect_core::Archetect;
use archetect_core::errors::ArchetectError;
use archetect_grpc::{ArchetectServer, ArchetectServiceCore};

pub fn handle_server_subcommand(archetect: Archetect) -> Result<(), ArchetectError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Viable Tokio Runtime");

    runtime
        .block_on(async {
            let core = ArchetectServiceCore::builder(archetect).build().await?;
            let server = ArchetectServer::builder(core).build().await?;

            tokio::select! {
                result = server.serve() => {
                  return result;
                },
                _ = tokio::signal::ctrl_c() => {
                    return Ok(());
                },
            }
        })
        .map_err(|err| ArchetectError::GeneralError(err.to_string()))?; //TODO: Create a better error

    Ok(())
}
