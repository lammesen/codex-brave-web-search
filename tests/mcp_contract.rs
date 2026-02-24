use codex_brave_web_search::config::RuntimeConfig;
use codex_brave_web_search::constants::{
    TOOL_BRAVE_WEB_SEARCH, TOOL_BRAVE_WEB_SEARCH_HELP, TOOL_BRAVE_WEB_SEARCH_STATUS,
};
use codex_brave_web_search::mcp_server::BraveSearchMcpServer;
use codex_brave_web_search::service::SearchService;
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
    let config = RuntimeConfig::from_env();
    let service = SearchService::new(config).expect("service should initialize");
    BraveSearchMcpServer::new(service)
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

fn parse_tool_error_json(result: mcpkit::types::tool::ToolOutput) -> serde_json::Value {
    let call_result: CallToolResult = result.into();
    assert_eq!(call_result.is_error, Some(true));

    let text = call_result
        .content
        .iter()
        .find_map(mcpkit::types::Content::as_text)
        .expect("tool error output should contain text");
    serde_json::from_str(text).expect("tool error output should be valid JSON")
}

#[tokio::test]
async fn lists_three_tools_with_expected_names() {
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

    let tools = server
        .list_tools(&ctx)
        .await
        .expect("list tools should work");
    let names = tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<&str>>();

    assert_eq!(names.len(), 3);
    assert!(names.contains(&TOOL_BRAVE_WEB_SEARCH));
    assert!(names.contains(&TOOL_BRAVE_WEB_SEARCH_HELP));
    assert!(names.contains(&TOOL_BRAVE_WEB_SEARCH_STATUS));

    // additionalProperties false for strict unknown-field rejection
    let search_tool = tools
        .iter()
        .find(|tool| tool.name == TOOL_BRAVE_WEB_SEARCH)
        .expect("search tool present");
    assert_eq!(
        search_tool.input_schema["additionalProperties"],
        serde_json::Value::Bool(false)
    );
}

#[tokio::test]
async fn help_tool_returns_structured_payload() {
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
            TOOL_BRAVE_WEB_SEARCH_HELP,
            serde_json::json!({"topic": "params"}),
            &ctx,
        )
        .await
        .expect("help tool should execute");

    let json = parse_tool_json(output);
    assert_eq!(json["topic"], "params");
    assert!(json["sections"]["parameters"].is_object());
    assert!(json["examples_markdown"].is_string());
}

#[tokio::test]
async fn help_topic_examples_returns_examples_without_param_sections() {
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
            TOOL_BRAVE_WEB_SEARCH_HELP,
            serde_json::json!({"topic": "examples"}),
            &ctx,
        )
        .await
        .expect("help tool should execute");

    let json = parse_tool_json(output);
    assert_eq!(json["topic"], "examples");
    assert_eq!(json["sections"]["parameters"], serde_json::json!({}));
    assert_eq!(json["sections"]["limits"], serde_json::json!({}));
    assert_eq!(json["sections"]["errors"], serde_json::json!({}));
    assert!(
        json["examples_markdown"]
            .as_str()
            .is_some_and(|s| s.contains("```json"))
    );
}

#[tokio::test]
async fn help_tool_invalid_args_returns_structured_error_payload() {
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
            TOOL_BRAVE_WEB_SEARCH_HELP,
            serde_json::json!({"topic": "invalid-topic"}),
            &ctx,
        )
        .await
        .expect("help tool should return structured error");

    let json = parse_tool_error_json(output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("brave_web_search_help"))
    );
}

#[tokio::test]
async fn status_tool_returns_runtime_shape() {
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
        .expect("status tool should execute");

    let json = parse_tool_json(output);
    assert!(json["status"].is_string());
    assert!(json["key_config"]["has_key"].is_boolean());
    assert!(json["settings"]["retry_count"].is_number());
    assert!(json["settings"]["limits"].is_object());
}

#[tokio::test]
async fn status_tool_invalid_args_returns_structured_error_payload() {
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
            serde_json::json!({"probe_connectivity": "yes"}),
            &ctx,
        )
        .await
        .expect("status tool should return structured error");

    let json = parse_tool_error_json(output);
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("brave_web_search_status"))
    );
}

#[tokio::test]
async fn search_tool_empty_query_returns_structured_error_payload() {
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
            serde_json::json!({"query": "   "}),
            &ctx,
        )
        .await
        .expect("tool call should execute");

    let call_result: CallToolResult = output.into();
    assert_eq!(call_result.is_error, Some(true));

    let text = call_result
        .content
        .iter()
        .find_map(mcpkit::types::Content::as_text)
        .expect("error payload text");
    let json: serde_json::Value = serde_json::from_str(text).expect("json envelope");
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(json["meta"]["trace_id"].is_string());
}

#[tokio::test]
async fn search_tool_invalid_search_type_returns_error_envelope() {
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
            serde_json::json!({"query": "openai", "search_type": "invalid"}),
            &ctx,
        )
        .await
        .expect("tool call should execute");

    let call_result: CallToolResult = output.into();
    assert_eq!(call_result.is_error, Some(true));

    let text = call_result
        .content
        .iter()
        .find_map(mcpkit::types::Content::as_text)
        .expect("error payload text");
    let json: serde_json::Value = serde_json::from_str(text).expect("json envelope");
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("search_type"))
    );
}

#[tokio::test]
async fn search_tool_all_invalid_result_filter_tokens_returns_error_envelope() {
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
            serde_json::json!({"query": "openai", "result_filter": ["unknown", "nope"]}),
            &ctx,
        )
        .await
        .expect("tool call should execute");

    let call_result: CallToolResult = output.into();
    assert_eq!(call_result.is_error, Some(true));

    let text = call_result
        .content
        .iter()
        .find_map(mcpkit::types::Content::as_text)
        .expect("error payload text");
    let json: serde_json::Value = serde_json::from_str(text).expect("json envelope");
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("result_filter"))
    );
}

#[tokio::test]
async fn unknown_parameter_is_rejected_by_schema_deserializer() {
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
            serde_json::json!({"query": "openai", "unexpected": true}),
            &ctx,
        )
        .await
        .expect("tool call should execute with structured error payload");

    let call_result: CallToolResult = output.into();
    assert_eq!(call_result.is_error, Some(true));
    let text = call_result
        .content
        .iter()
        .find_map(mcpkit::types::Content::as_text)
        .expect("error payload text");
    let json: serde_json::Value = serde_json::from_str(text).expect("json envelope");
    assert_eq!(json["error"]["code"], "INVALID_ARGUMENT");
    assert!(
        json["error"]["details"]["reason"]
            .as_str()
            .is_some_and(|message| message.contains("unknown field"))
    );
}
