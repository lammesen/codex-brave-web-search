use codex_brave_web_search::config::RuntimeConfig;
use codex_brave_web_search::constants::{
    TOOL_BRAVE_WEB_SEARCH, TOOL_BRAVE_WEB_SEARCH_HELP, TOOL_BRAVE_WEB_SEARCH_STATUS,
};
use codex_brave_web_search::mcp_server::BraveSearchMcpServer;
use codex_brave_web_search::service::SearchService;
use insta::assert_json_snapshot;
use mcpkit::capability::{ClientCapabilities, ServerCapabilities};
use mcpkit::protocol::RequestId;
use mcpkit::protocol_version::ProtocolVersion;
use mcpkit::types::tool::CallToolResult;
use mcpkit::{Context, NoOpPeer, ToolHandler};

fn make_context() -> (
    RequestId,
    ClientCapabilities,
    ServerCapabilities,
    ProtocolVersion,
    NoOpPeer,
) {
    (
        RequestId::Number(1),
        ClientCapabilities::default(),
        ServerCapabilities::default(),
        ProtocolVersion::LATEST,
        NoOpPeer,
    )
}

fn make_server() -> BraveSearchMcpServer {
    temp_env::with_var("BRAVE_SEARCH_API_KEY", Some("test-key"), || {
        temp_env::with_var("BRAVE_API_KEY", None::<&str>, || {
            let config = RuntimeConfig::from_env();
            let service = SearchService::new(config).expect("service should initialize");
            BraveSearchMcpServer::new(service)
        })
    })
}

fn parse_tool_json(result: mcpkit::types::tool::ToolOutput) -> serde_json::Value {
    let call_result: CallToolResult = result.into();
    let text = call_result
        .content
        .iter()
        .find_map(mcpkit::types::Content::as_text)
        .expect("tool output should contain text");
    serde_json::from_str(text).expect("tool output should be valid JSON")
}

#[tokio::test]
async fn snapshot_help_all() {
    let server = make_server();
    let (req_id, client_caps, server_caps, protocol_version, peer) = make_context();
    let ctx = Context::new(
        &req_id,
        None,
        &client_caps,
        &server_caps,
        protocol_version,
        &peer,
    );

    let output = server
        .call_tool(TOOL_BRAVE_WEB_SEARCH_HELP, serde_json::json!({}), &ctx)
        .await
        .expect("help call should succeed");
    let json = parse_tool_json(output);

    assert_json_snapshot!("help_all", json);
}

#[tokio::test]
async fn snapshot_status_no_probe() {
    let server = make_server();
    let (req_id, client_caps, server_caps, protocol_version, peer) = make_context();
    let ctx = Context::new(
        &req_id,
        None,
        &client_caps,
        &server_caps,
        protocol_version,
        &peer,
    );

    let output = server
        .call_tool(
            TOOL_BRAVE_WEB_SEARCH_STATUS,
            serde_json::json!({"include_limits": true}),
            &ctx,
        )
        .await
        .expect("status call should succeed");

    let mut json = parse_tool_json(output);
    json["server_version"] = serde_json::json!("<version>");
    assert_json_snapshot!("status_no_probe", json);
}

#[tokio::test]
async fn snapshot_search_error_envelope() {
    let server = make_server();
    let (req_id, client_caps, server_caps, protocol_version, peer) = make_context();
    let ctx = Context::new(
        &req_id,
        None,
        &client_caps,
        &server_caps,
        protocol_version,
        &peer,
    );

    let output = server
        .call_tool(
            TOOL_BRAVE_WEB_SEARCH,
            serde_json::json!({"query": ""}),
            &ctx,
        )
        .await
        .expect("call should return tool error envelope");

    let mut json = parse_tool_json(output);
    json["meta"]["trace_id"] = serde_json::json!("<trace>");
    json["meta"]["server_version"] = serde_json::json!("<version>");
    assert_json_snapshot!("search_error_envelope", json);
}
