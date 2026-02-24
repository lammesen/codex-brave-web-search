use crate::constants::{
    TOOL_BRAVE_WEB_SEARCH, TOOL_BRAVE_WEB_SEARCH_HELP, TOOL_BRAVE_WEB_SEARCH_STATUS,
};
use crate::error::AppError;
use crate::service::SearchService;
use crate::types::{BraveWebSearchArgs, HelpArgs, StatusArgs};
use mcpkit::capability::{ServerCapabilities, ServerInfo};
use mcpkit::error::McpError;
use mcpkit::types::content::Content;
use mcpkit::types::tool::{CallToolResult, Tool, ToolAnnotations, ToolOutput};
use mcpkit::{Context, ServerHandler, ToolHandler};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BraveSearchMcpServer {
    service: Arc<SearchService>,
}

impl BraveSearchMcpServer {
    #[must_use]
    pub fn new(service: SearchService) -> Self {
        Self {
            service: Arc::new(service),
        }
    }

    fn tools() -> Vec<Tool> {
        vec![
            search_tool_schema(),
            help_tool_schema(),
            status_tool_schema(),
        ]
    }
}

impl ServerHandler for BraveSearchMcpServer {
    fn server_info(&self) -> ServerInfo {
        ServerInfo::new("brave-web-search", self.service.server_version())
    }

    fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities::new().with_tools()
    }

    fn instructions(&self) -> Option<String> {
        Some(
            "Use brave_web_search for Brave web/news/images/videos queries. Use brave_web_search_help for schema/examples and brave_web_search_status for config/health checks.".to_string(),
        )
    }
}

impl ToolHandler for BraveSearchMcpServer {
    async fn list_tools(&self, _ctx: &Context<'_>) -> Result<Vec<Tool>, McpError> {
        Ok(Self::tools())
    }

    async fn call_tool(
        &self,
        name: &str,
        args: Value,
        ctx: &Context<'_>,
    ) -> Result<ToolOutput, McpError> {
        let trace_id = Uuid::new_v4().to_string();

        match name {
            TOOL_BRAVE_WEB_SEARCH => {
                let parsed = match parse_tool_args::<BraveWebSearchArgs>(args, name) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        return Ok(error_tool_output(
                            &error,
                            self.service.server_version(),
                            &trace_id,
                        ));
                    }
                };
                match self
                    .service
                    .execute_web_search(parsed, &trace_id, || ctx.is_cancelled())
                    .await
                {
                    Ok(response) => json_tool_output(&response),
                    Err(error) => Ok(error_tool_output(
                        &error,
                        self.service.server_version(),
                        &trace_id,
                    )),
                }
            }
            TOOL_BRAVE_WEB_SEARCH_HELP => {
                let parsed = match parse_tool_args::<HelpArgs>(args, name) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        return Ok(error_tool_output(
                            &error,
                            self.service.server_version(),
                            &trace_id,
                        ));
                    }
                };
                let response = self.service.help(parsed.topic);
                json_tool_output(&response)
            }
            TOOL_BRAVE_WEB_SEARCH_STATUS => {
                let parsed = match parse_tool_args::<StatusArgs>(args, name) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        return Ok(error_tool_output(
                            &error,
                            self.service.server_version(),
                            &trace_id,
                        ));
                    }
                };
                let response = self.service.status(parsed, || ctx.is_cancelled()).await;
                json_tool_output(&response)
            }
            _ => Err(McpError::invalid_params(
                "tools/call",
                format!("Unknown tool: {name}"),
            )),
        }
    }
}

fn normalize_args(value: Value) -> Value {
    match value {
        Value::Null => Value::Object(serde_json::Map::new()),
        other => other,
    }
}

fn parse_tool_args<T>(value: Value, tool_name: &str) -> Result<T, AppError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(normalize_args(value)).map_err(|error| {
        AppError::invalid_argument_with_details(
            format!("Invalid arguments for {tool_name}"),
            serde_json::json!({ "reason": error.to_string() }),
        )
    })
}

fn json_tool_output<T: Serialize>(value: &T) -> Result<ToolOutput, McpError> {
    let json = serde_json::to_string_pretty(value).map_err(|error| {
        McpError::internal(format!("Failed to serialize tool response: {error}"))
    })?;

    Ok(ToolOutput::Success(CallToolResult {
        content: vec![Content::text(json)],
        is_error: None,
    }))
}

fn error_tool_output(error: &AppError, server_version: &str, trace_id: &str) -> ToolOutput {
    let envelope = error.to_envelope(server_version, trace_id);
    let payload = serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| {
        format!(
            "{{\"api_version\":\"v1\",\"error\":{{\"code\":\"{}\",\"message\":\"{}\"}},\"meta\":{{\"trace_id\":\"{}\"}}}}",
            error.code(),
            error.message().replace('"', "\\\""),
            trace_id
        )
    });

    ToolOutput::Success(CallToolResult {
        content: vec![Content::text(payload)],
        is_error: Some(true),
    })
}

fn search_tool_schema() -> Tool {
    Tool::new(TOOL_BRAVE_WEB_SEARCH)
        .description("Search Brave web/news/images/videos endpoints with structured JSON output and diagnostics")
        .input_schema(serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["query"],
            "properties": {
                "query": { "type": "string", "description": "Search query." },
                "search_type": { "type": "string", "enum": ["web", "news", "images", "videos"] },
                "result_filter": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Web-only section filters; supported values: web, discussions, videos, news, infobox"
                },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 20 },
                "offset": { "type": "integer", "minimum": 0 },
                "country": { "type": "string" },
                "search_language": { "type": "string" },
                "ui_language": { "type": "string" },
                "safe_search": { "type": "string", "description": "off | moderate | strict" },
                "units": { "type": "string", "description": "metric | imperial" },
                "freshness": { "type": "string" },
                "spellcheck": { "type": "boolean" },
                "extra_snippets": { "type": "boolean" },
                "text_decorations": { "type": "boolean" },
                "max_lines": { "type": "integer", "minimum": 1 },
                "max_bytes": { "type": "integer", "minimum": 1 },
                "debug": { "type": "boolean" },
                "include_raw_payload": { "type": "boolean" },
                "disable_cache": { "type": "boolean" },
                "disable_throttle": { "type": "boolean" },
                "include_request_url": { "type": "boolean" }
            }
        }))
        .annotations(ToolAnnotations::read_only())
}

fn help_tool_schema() -> Tool {
    Tool::new(TOOL_BRAVE_WEB_SEARCH_HELP)
        .description("Show parameter, limits, and error guidance for brave_web_search")
        .input_schema(serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "topic": {
                    "type": "string",
                    "enum": ["params", "examples", "limits", "errors", "all"]
                }
            }
        }))
        .annotations(ToolAnnotations::read_only())
}

fn status_tool_schema() -> Tool {
    Tool::new(TOOL_BRAVE_WEB_SEARCH_STATUS)
        .description("Show server runtime status and optional Brave endpoint connectivity probes")
        .input_schema(serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "probe_connectivity": { "type": "boolean", "default": false },
                "verbose": { "type": "boolean", "default": false },
                "include_limits": { "type": "boolean", "default": false }
            }
        }))
        .annotations(ToolAnnotations::read_only())
}
