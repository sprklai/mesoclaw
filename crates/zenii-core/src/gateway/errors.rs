use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tracing::error;

use crate::ZeniiError;

#[derive(Debug, Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Sub-classify `ZeniiError::Agent` messages into specific error codes.
fn classify_agent_error_code(msg: &str) -> &'static str {
    let lower = msg.to_lowercase();
    if lower.contains("maxturn") || lower.contains("max turn") {
        "ZENII_AGENT_MAX_TURNS"
    } else if lower.contains("401")
        || lower.contains("unauthorized")
        || lower.contains("invalid api key")
        || lower.contains("authentication")
    {
        "ZENII_AGENT_AUTH"
    } else if lower.contains("429")
        || lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("too many requests")
    {
        "ZENII_AGENT_RATE_LIMIT"
    } else if lower.contains("context length")
        || lower.contains("too many tokens")
        || lower.contains("token limit")
    {
        "ZENII_AGENT_CONTEXT_LENGTH"
    } else if lower.contains("model")
        && (lower.contains("not found") || lower.contains("does not exist"))
    {
        "ZENII_AGENT_MODEL_NOT_FOUND"
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "ZENII_AGENT_TIMEOUT"
    } else if lower.contains("connection refused")
        || lower.contains("connect error")
        || lower.contains("dns")
    {
        "ZENII_AGENT_CONNECTION"
    } else if lower.contains("no agent configured") || lower.contains("no provider") {
        "ZENII_AGENT_NOT_CONFIGURED"
    } else {
        "ZENII_AGENT_ERROR"
    }
}

/// Map an agent sub-classified error code to a specific HTTP status.
fn status_for_agent_code(code: &str) -> StatusCode {
    match code {
        "ZENII_AGENT_AUTH" => StatusCode::UNAUTHORIZED,
        "ZENII_AGENT_RATE_LIMIT" => StatusCode::TOO_MANY_REQUESTS,
        "ZENII_AGENT_MAX_TURNS" | "ZENII_AGENT_CONTEXT_LENGTH" | "ZENII_AGENT_MODEL_NOT_FOUND" => {
            StatusCode::BAD_REQUEST
        }
        "ZENII_AGENT_TIMEOUT" => StatusCode::GATEWAY_TIMEOUT,
        "ZENII_AGENT_CONNECTION" => StatusCode::BAD_GATEWAY,
        "ZENII_AGENT_NOT_CONFIGURED" => StatusCode::SERVICE_UNAVAILABLE,
        // "ZENII_AGENT_ERROR" and anything else
        _ => StatusCode::BAD_GATEWAY,
    }
}

/// Return the canonical error code for a `ZeniiError`.
///
/// For `Agent` errors this sub-classifies based on message content.
pub fn error_code_for(err: &ZeniiError) -> &'static str {
    match err {
        ZeniiError::NotFound(_) => "ZENII_NOT_FOUND",
        ZeniiError::PolicyDenied(_) => "ZENII_POLICY_DENIED",
        ZeniiError::RateLimited(_) => "ZENII_RATE_LIMITED",
        ZeniiError::Auth(_) => "ZENII_AUTH_REQUIRED",
        ZeniiError::Serialization(_) => "ZENII_BAD_REQUEST",
        ZeniiError::TomlParse(_) => "ZENII_TOML_PARSE_ERROR",
        ZeniiError::Config(_) => "ZENII_CONFIG_ERROR",
        ZeniiError::Database(_) => "ZENII_DB_ERROR",
        ZeniiError::Sqlite(_) => "ZENII_SQLITE_ERROR",
        ZeniiError::Agent(msg) => classify_agent_error_code(msg),
        ZeniiError::Http(_) => "ZENII_HTTP_ERROR",
        ZeniiError::Tool(_) => "ZENII_TOOL_ERROR",
        ZeniiError::Memory(_) => "ZENII_MEMORY_ERROR",
        ZeniiError::MemoryDuplicate(_) => "ZENII_MEMORY_DUPLICATE",
        ZeniiError::Embedding(_) => "ZENII_EMBEDDING_ERROR",
        ZeniiError::Credential(_) => "ZENII_CREDENTIAL_ERROR",
        ZeniiError::Gateway(_) => "ZENII_GATEWAY_ERROR",
        ZeniiError::Io(_) => "ZENII_IO_ERROR",
        ZeniiError::EventBus(_) => "ZENII_EVENT_ERROR",
        ZeniiError::Channel(_) => "ZENII_CHANNEL_ERROR",
        ZeniiError::Context(_) => "ZENII_CONTEXT",
        ZeniiError::TomlSerialize(_) => "ZENII_TOML_SERIALIZE_ERROR",
        ZeniiError::Identity(_) => "ZENII_IDENTITY_ERROR",
        ZeniiError::IdentityNotFound(_) => "ZENII_IDENTITY_NOT_FOUND",
        ZeniiError::Skill(_) => "ZENII_SKILL_ERROR",
        ZeniiError::SkillNotFound(_) => "ZENII_SKILL_NOT_FOUND",
        ZeniiError::User(_) => "ZENII_USER_ERROR",
        ZeniiError::Yaml(_) => "ZENII_YAML_PARSE_ERROR",
        ZeniiError::Validation(_) => "ZENII_VALIDATION",
        ZeniiError::Scheduler(_) => "ZENII_SCHEDULER_ERROR",
        ZeniiError::Plugin(_) => "ZENII_PLUGIN_ERROR",
        ZeniiError::PluginNotFound(_) => "ZENII_PLUGIN_NOT_FOUND",
        ZeniiError::Workflow(_) => "ZENII_WORKFLOW_ERROR",
        ZeniiError::ModelCapability(_) => "ZENII_MODEL_CAPABILITY",
        ZeniiError::ApprovalDenied(_) => "ZENII_APPROVAL_DENIED",
        ZeniiError::ApprovalTimeout(_) => "ZENII_APPROVAL_TIMEOUT",
        ZeniiError::Mcp(_) => "ZENII_MCP_ERROR",
        ZeniiError::Conversion(_) => "ZENII_CONVERSION",
        ZeniiError::Other(_) => "ZENII_INTERNAL_ERROR",
    }
}

/// Return the HTTP status code for a `ZeniiError`.
///
/// For `Agent` errors this derives the status from the sub-classified code.
fn status_code_for(err: &ZeniiError) -> StatusCode {
    match err {
        ZeniiError::NotFound(_) => StatusCode::NOT_FOUND,
        ZeniiError::PolicyDenied(_) => StatusCode::FORBIDDEN,
        ZeniiError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
        ZeniiError::Auth(_) => StatusCode::UNAUTHORIZED,
        ZeniiError::Serialization(_) => StatusCode::BAD_REQUEST,
        ZeniiError::TomlParse(_) => StatusCode::BAD_REQUEST,
        ZeniiError::Config(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ZeniiError::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
        ZeniiError::Sqlite(_) => StatusCode::SERVICE_UNAVAILABLE,
        ZeniiError::Agent(_) => status_for_agent_code(error_code_for(err)),
        ZeniiError::Http(_) => StatusCode::BAD_GATEWAY,
        ZeniiError::Tool(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Memory(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::MemoryDuplicate(_) => StatusCode::CONFLICT,
        ZeniiError::Embedding(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Credential(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Gateway(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::EventBus(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Channel(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Context(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::TomlSerialize(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Identity(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::IdentityNotFound(_) => StatusCode::NOT_FOUND,
        ZeniiError::Skill(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::SkillNotFound(_) => StatusCode::NOT_FOUND,
        ZeniiError::User(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Yaml(_) => StatusCode::BAD_REQUEST,
        ZeniiError::Validation(_) => StatusCode::BAD_REQUEST,
        ZeniiError::Scheduler(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Plugin(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::PluginNotFound(_) => StatusCode::NOT_FOUND,
        ZeniiError::Workflow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::ModelCapability(_) => StatusCode::BAD_REQUEST,
        ZeniiError::ApprovalDenied(_) => StatusCode::FORBIDDEN,
        ZeniiError::ApprovalTimeout(_) => StatusCode::REQUEST_TIMEOUT,
        ZeniiError::Mcp(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ZeniiError::Conversion(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ZeniiError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl IntoResponse for ZeniiError {
    fn into_response(self) -> Response {
        let error_code = error_code_for(&self);
        let status = status_code_for(&self);

        // Sanitize internal error messages to prevent info leakage.
        // Log the detailed error server-side, return generic message to client.
        let message = match &self {
            ZeniiError::Sqlite(_) | ZeniiError::Database(_) | ZeniiError::Io(_) => {
                error!("Internal error ({}): {}", error_code, self);
                "Internal server error".to_string()
            }
            _ => self.to_string(),
        };

        let hint = crate::error::enrich_error(&self).map(|h| h.action);

        let body = ErrorResponse {
            error_code: error_code.to_string(),
            message,
            hint,
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use std::collections::HashSet;

    /// Helper to extract status, error_code, message, and hint from a ZeniiError response.
    fn response_parts_full(err: ZeniiError) -> (StatusCode, String, String, Option<String>) {
        let response = err.into_response();
        let status = response.status();

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("test runtime");
        let body_bytes = rt.block_on(async {
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read body")
        });
        let error_resp: ErrorResponse =
            serde_json::from_slice(&body_bytes).expect("parse error response");
        (
            status,
            error_resp.error_code.to_string(),
            error_resp.message,
            error_resp.hint,
        )
    }

    fn response_parts(err: ZeniiError) -> (StatusCode, String) {
        let (status, code, _, _) = response_parts_full(err);
        (status, code)
    }

    #[test]
    fn not_found_maps_to_404() {
        let (status, code) = response_parts(ZeniiError::NotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "ZENII_NOT_FOUND");
    }

    #[test]
    fn policy_denied_maps_to_403() {
        let (status, code) = response_parts(ZeniiError::PolicyDenied("blocked".into()));
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(code, "ZENII_POLICY_DENIED");
    }

    #[test]
    fn rate_limited_maps_to_429() {
        let (status, code) = response_parts(ZeniiError::RateLimited("slow down".into()));
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(code, "ZENII_RATE_LIMITED");
    }

    #[test]
    fn auth_maps_to_401() {
        let (status, code) = response_parts(ZeniiError::Auth("no token".into()));
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "ZENII_AUTH_REQUIRED");
    }

    #[test]
    fn serialization_maps_to_400() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let (status, code) = response_parts(ZeniiError::Serialization(json_err));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(code, "ZENII_BAD_REQUEST");
    }

    #[test]
    fn config_maps_to_422() {
        let (status, code) = response_parts(ZeniiError::Config("bad config".into()));
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(code, "ZENII_CONFIG_ERROR");
    }

    #[test]
    fn database_maps_to_503() {
        let (status, code) = response_parts(ZeniiError::Database("db down".into()));
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "ZENII_DB_ERROR");
    }

    #[test]
    fn agent_maps_to_502() {
        let (status, code) = response_parts(ZeniiError::Agent("agent failed".into()));
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(code, "ZENII_AGENT_ERROR");
    }

    #[test]
    fn tool_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::Tool("tool broke".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_TOOL_ERROR");
    }

    #[test]
    fn other_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::Other("unknown".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_INTERNAL_ERROR");
    }

    #[test]
    fn gateway_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::Gateway("gw error".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_GATEWAY_ERROR");
    }

    #[test]
    fn all_error_codes_unique() {
        // Collect all error codes by creating one of each variant.
        // Agent("t") maps to ZENII_AGENT_ERROR (generic fallback).
        let json_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
        let toml_err = toml::from_str::<toml::Value>("= bad").unwrap_err();
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");

        let errors: Vec<ZeniiError> = vec![
            ZeniiError::NotFound("t".into()),
            ZeniiError::PolicyDenied("t".into()),
            ZeniiError::RateLimited("t".into()),
            ZeniiError::Auth("t".into()),
            ZeniiError::Serialization(json_err),
            ZeniiError::TomlParse(toml_err),
            ZeniiError::Config("t".into()),
            ZeniiError::Database("t".into()),
            ZeniiError::Sqlite(rusqlite::Error::InvalidParameterName("t".into())),
            ZeniiError::Agent("t".into()),
            // Skip Http — requires a real reqwest::Error which is hard to construct
            ZeniiError::Tool("t".into()),
            ZeniiError::Memory("t".into()),
            ZeniiError::Embedding("t".into()),
            ZeniiError::Credential("t".into()),
            ZeniiError::Gateway("t".into()),
            ZeniiError::Io(io_err),
            ZeniiError::EventBus("t".into()),
            ZeniiError::Channel("t".into()),
            ZeniiError::Context("t".into()),
            {
                use serde::ser::Error as _;
                ZeniiError::TomlSerialize(toml::ser::Error::custom("t"))
            },
            ZeniiError::Identity("t".into()),
            ZeniiError::IdentityNotFound("t".into()),
            ZeniiError::Skill("t".into()),
            ZeniiError::SkillNotFound("t".into()),
            ZeniiError::User("t".into()),
            {
                let yaml_err =
                    serde_yaml::from_str::<serde_yaml::Value>(": bad: yaml:").unwrap_err();
                ZeniiError::Yaml(yaml_err)
            },
            ZeniiError::Validation("t".into()),
            ZeniiError::Scheduler("t".into()),
            ZeniiError::Plugin("t".into()),
            ZeniiError::PluginNotFound("t".into()),
            ZeniiError::Workflow("t".into()),
            ZeniiError::ModelCapability("t".into()),
            ZeniiError::ApprovalDenied("t".into()),
            ZeniiError::ApprovalTimeout("t".into()),
            ZeniiError::Other("t".into()),
        ];

        let mut codes = HashSet::new();
        for err in errors {
            let (_, code) = response_parts(err);
            assert!(codes.insert(code.clone()), "duplicate error code: {code}");
        }

        // 35 variants tested (Http skipped because reqwest::Error can't be easily constructed)
        assert_eq!(codes.len(), 35);
    }

    #[test]
    fn identity_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::Identity("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_IDENTITY_ERROR");
    }

    #[test]
    fn identity_not_found_maps_to_404() {
        let (status, code) = response_parts(ZeniiError::IdentityNotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "ZENII_IDENTITY_NOT_FOUND");
    }

    #[test]
    fn skill_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::Skill("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_SKILL_ERROR");
    }

    #[test]
    fn skill_not_found_maps_to_404() {
        let (status, code) = response_parts(ZeniiError::SkillNotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "ZENII_SKILL_NOT_FOUND");
    }

    #[test]
    fn user_error_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::User("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_USER_ERROR");
    }

    #[test]
    fn yaml_error_maps_to_400() {
        let yaml_err = serde_yaml::from_str::<serde_yaml::Value>(": bad").unwrap_err();
        let (status, code) = response_parts(ZeniiError::Yaml(yaml_err));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(code, "ZENII_YAML_PARSE_ERROR");
    }

    // --- Agent sub-classification tests ---

    #[test]
    fn agent_error_sub_classified() {
        // MaxTurnError → ZENII_AGENT_MAX_TURNS, 400
        let (status, code) =
            response_parts(ZeniiError::Agent("MaxTurnError: reached limit".into()));
        assert_eq!(code, "ZENII_AGENT_MAX_TURNS");
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // 401 Unauthorized → ZENII_AGENT_AUTH, 401
        let (status, code) =
            response_parts(ZeniiError::Agent("ProviderError: 401 Unauthorized".into()));
        assert_eq!(code, "ZENII_AGENT_AUTH");
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // 429 rate limit → ZENII_AGENT_RATE_LIMIT, 429
        let (status, code) = response_parts(ZeniiError::Agent("429 rate limit exceeded".into()));
        assert_eq!(code, "ZENII_AGENT_RATE_LIMIT");
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

        // context length → ZENII_AGENT_CONTEXT_LENGTH, 400
        let (status, code) = response_parts(ZeniiError::Agent("context length exceeded".into()));
        assert_eq!(code, "ZENII_AGENT_CONTEXT_LENGTH");
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // model not found → ZENII_AGENT_MODEL_NOT_FOUND, 400
        let (status, code) =
            response_parts(ZeniiError::Agent("model gpt-99 does not exist".into()));
        assert_eq!(code, "ZENII_AGENT_MODEL_NOT_FOUND");
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // timeout → ZENII_AGENT_TIMEOUT, 504
        let (status, code) = response_parts(ZeniiError::Agent("request timed out".into()));
        assert_eq!(code, "ZENII_AGENT_TIMEOUT");
        assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);

        // connection refused → ZENII_AGENT_CONNECTION, 502
        let (status, code) = response_parts(ZeniiError::Agent("connection refused".into()));
        assert_eq!(code, "ZENII_AGENT_CONNECTION");
        assert_eq!(status, StatusCode::BAD_GATEWAY);

        // no provider → ZENII_AGENT_NOT_CONFIGURED, 503
        let (status, code) = response_parts(ZeniiError::Agent("no agent configured".into()));
        assert_eq!(code, "ZENII_AGENT_NOT_CONFIGURED");
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

        // generic fallback → ZENII_AGENT_ERROR, 502
        let (status, code) = response_parts(ZeniiError::Agent("something weird happened".into()));
        assert_eq!(code, "ZENII_AGENT_ERROR");
        assert_eq!(status, StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn error_response_has_hint() {
        // Agent("401") should produce a hint via enrich_error
        let (_, _, _, hint) =
            response_parts_full(ZeniiError::Agent("ProviderError: 401 Unauthorized".into()));
        assert!(hint.is_some(), "Agent auth error should include a hint");
        assert!(
            hint.unwrap().contains("API key"),
            "Hint should mention API key"
        );
    }

    #[test]
    fn error_response_no_hint_for_generic() {
        // A generic Agent error with no matching pattern should have no hint
        let (_, _, _, hint) =
            response_parts_full(ZeniiError::Agent("some random internal error".into()));
        assert!(
            hint.is_none(),
            "Generic agent error should not include a hint"
        );
    }

    // --- WS-4.5: Error message sanitization ---

    #[test]
    fn sqlite_error_does_not_leak_path() {
        let err = ZeniiError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("/home/user/.local/share/zenii/data.db: disk I/O error".into()),
        ));
        let (status, code, message, _) = response_parts_full(err);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "ZENII_SQLITE_ERROR");
        assert!(
            message.contains("Internal"),
            "SQLite error should return generic message, got: {message}"
        );
        assert!(
            !message.contains("/home"),
            "SQLite error should not leak file paths, got: {message}"
        );
    }

    #[test]
    fn database_error_does_not_leak_details() {
        let err = ZeniiError::Database("connection to /var/db/zenii.db failed: SQLITE_BUSY".into());
        let (_, _, message, _) = response_parts_full(err);
        assert!(
            message.contains("Internal"),
            "Database error should return generic message, got: {message}"
        );
        assert!(
            !message.contains("/var/db"),
            "Database error should not leak file paths, got: {message}"
        );
    }

    #[test]
    fn io_error_does_not_leak_path() {
        let err = ZeniiError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "/home/user/secret/file.txt not found",
        ));
        let (_, _, message, _) = response_parts_full(err);
        assert!(
            message.contains("Internal"),
            "IO error should return generic message, got: {message}"
        );
        assert!(
            !message.contains("/home"),
            "IO error should not leak file paths, got: {message}"
        );
    }

    #[test]
    fn not_found_still_returns_details() {
        let err = ZeniiError::NotFound("session abc-123".into());
        let (_, _, message, _) = response_parts_full(err);
        assert!(
            message.contains("abc-123"),
            "NotFound should still include details for debugging"
        );
    }

    #[test]
    fn hint_is_omitted_from_json_when_none() {
        // Errors with no hint should not have the "hint" key in JSON at all
        let err = ZeniiError::NotFound("missing".into());
        let response = err.into_response();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("test runtime");
        let body_bytes = rt.block_on(async {
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read body")
        });
        let raw: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse json");
        assert!(
            raw.get("hint").is_none(),
            "hint field should be omitted from JSON when None, got: {raw}"
        );
    }
}
