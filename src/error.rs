use crate::constants::{
    API_VERSION, ERROR_CANCELLED, ERROR_INTERNAL, ERROR_INVALID_ARGUMENT, ERROR_MISSING_API_KEY,
    ERROR_PARSE, ERROR_UPSTREAM, PROVIDER_NAME,
};
use crate::types::{ErrorMeta, ToolErrorEnvelope, ToolErrorInfo};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("invalid argument: {message}")]
    InvalidArgument {
        message: String,
        details: Option<serde_json::Value>,
    },
    #[error("missing API key; set BRAVE_SEARCH_API_KEY or BRAVE_API_KEY")]
    MissingApiKey,
    #[error("request cancelled")]
    Cancelled,
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument { .. } => ERROR_INVALID_ARGUMENT,
            Self::MissingApiKey => ERROR_MISSING_API_KEY,
            Self::Cancelled => ERROR_CANCELLED,
            Self::Upstream(_) => ERROR_UPSTREAM,
            Self::Parse(_) => ERROR_PARSE,
            Self::Internal(_) => ERROR_INTERNAL,
        }
    }

    #[must_use]
    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            Self::InvalidArgument { details, .. } => details.clone(),
            _ => None,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::InvalidArgument { message, .. } => message.clone(),
            Self::MissingApiKey => {
                "Missing BRAVE_SEARCH_API_KEY/BRAVE_API_KEY. Configure env vars for MCP launch."
                    .to_string()
            }
            Self::Cancelled => "Search cancelled.".to_string(),
            Self::Upstream(message) => message.clone(),
            Self::Parse(message) => message.clone(),
            Self::Internal(message) => message.clone(),
        }
    }

    #[must_use]
    pub fn to_envelope(&self, server_version: &str, trace_id: &str) -> ToolErrorEnvelope {
        ToolErrorEnvelope {
            api_version: API_VERSION.to_string(),
            error: ToolErrorInfo {
                code: self.code().to_string(),
                message: self.message(),
                details: self.details(),
            },
            meta: ErrorMeta {
                provider: PROVIDER_NAME.to_string(),
                server_version: server_version.to_string(),
                trace_id: trace_id.to_string(),
            },
        }
    }

    #[must_use]
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
            details: None,
        }
    }

    #[must_use]
    pub fn invalid_argument_with_details(
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::InvalidArgument {
            message: message.into(),
            details: Some(details),
        }
    }
}
