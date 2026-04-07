use archetect_core::errors::ArchetectError;
use archetect_core::Archetect;

pub fn handle_mcp_subcommand(archetect: Archetect) -> Result<(), ArchetectError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        archetect_mcp::serve_stdio(archetect).await
    })
    .map_err(|err| ArchetectError::GeneralError(format!("MCP server error: {}", err)))
}
