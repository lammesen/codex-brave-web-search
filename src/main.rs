use codex_brave_web_search::config::RuntimeConfig;
use codex_brave_web_search::mcp_server::BraveSearchMcpServer;
use codex_brave_web_search::service::SearchService;
use mcpkit::ServerBuilder;
use mcpkit::error::McpError;
use mcpkit::transport::stdio::StdioTransport;

#[tokio::main]
async fn main() -> Result<(), McpError> {
    let config = RuntimeConfig::from_env();

    tracing_subscriber::fmt()
        .with_env_filter(config.log_filter.clone())
        .with_writer(std::io::stderr)
        .init();

    let service = SearchService::new(config)
        .map_err(|error| McpError::internal(format!("startup: {error}")))?;

    let handler = BraveSearchMcpServer::new(service);
    let server = ServerBuilder::new(handler.clone())
        .with_tools(handler)
        .build();
    server.serve(StdioTransport::new()).await
}
