mod io_handle;
mod prompt_envelope;
mod server;
mod session;

use rmcp::ServiceExt;
use rmcp::transport::stdio;

use archetect_core::Archetect;

pub use server::ArchetectMcpServer;

/// Start the MCP stdio server. This blocks until the client disconnects.
pub async fn serve_stdio(archetect: Archetect) -> Result<(), Box<dyn std::error::Error>> {
    let server = ArchetectMcpServer::new(archetect);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
