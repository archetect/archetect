use archetect_core::archetype::render_context::RenderContext;
use archetect_core::errors::ArchetectError;

pub fn handle_connect_subcommand(render_context: RenderContext) -> Result<(), ArchetectError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Viable Tokio Runtime");

    runtime
        .block_on(async move {
            return archetect_grpc::client::start(render_context).await;
            // tokio::select! {
            //     result = archetect_grpc::client::start(render_context) => {
            //       return result;
            //     },
            //     _ = tokio::signal::ctrl_c() => {
            //         return Ok(());
            //     },
            // }
        })
        .map_err(|err| ArchetectError::GeneralError(err.to_string()))?; //TODO: Create a better error

    Ok(())
}
