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
}

impl IntoResponse for ZeniiError {
    fn into_response(self) -> Response {
        let (status, error_code) = match &self {
            ZeniiError::NotFound(_) => (StatusCode::NOT_FOUND, "ZENII_NOT_FOUND"),
            ZeniiError::PolicyDenied(_) => (StatusCode::FORBIDDEN, "ZENII_POLICY_DENIED"),
            ZeniiError::RateLimited(_) => (StatusCode::TOO_MANY_REQUESTS, "ZENII_RATE_LIMITED"),
            ZeniiError::Auth(_) => (StatusCode::UNAUTHORIZED, "ZENII_AUTH_REQUIRED"),
            ZeniiError::Serialization(_) => (StatusCode::BAD_REQUEST, "ZENII_BAD_REQUEST"),
            ZeniiError::TomlParse(_) => (StatusCode::BAD_REQUEST, "ZENII_TOML_PARSE_ERROR"),
            ZeniiError::Config(_) => (StatusCode::UNPROCESSABLE_ENTITY, "ZENII_CONFIG_ERROR"),
            ZeniiError::Database(_) => (StatusCode::SERVICE_UNAVAILABLE, "ZENII_DB_ERROR"),
            ZeniiError::Sqlite(_) => (StatusCode::SERVICE_UNAVAILABLE, "ZENII_SQLITE_ERROR"),
            ZeniiError::Agent(_) => (StatusCode::BAD_GATEWAY, "ZENII_AGENT_ERROR"),
            ZeniiError::Http(_) => (StatusCode::BAD_GATEWAY, "ZENII_HTTP_ERROR"),
            ZeniiError::Tool(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_TOOL_ERROR"),
            ZeniiError::Memory(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_MEMORY_ERROR"),
            ZeniiError::Embedding(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_EMBEDDING_ERROR")
            }
            ZeniiError::Credential(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_CREDENTIAL_ERROR")
            }
            ZeniiError::Gateway(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_GATEWAY_ERROR"),
            ZeniiError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_IO_ERROR"),
            ZeniiError::EventBus(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_EVENT_ERROR"),
            ZeniiError::Channel(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_CHANNEL_ERROR"),
            ZeniiError::Context(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_CONTEXT"),
            ZeniiError::TomlSerialize(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "ZENII_TOML_SERIALIZE_ERROR",
            ),
            ZeniiError::Identity(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_P4_IDENTITY"),
            ZeniiError::IdentityNotFound(_) => {
                (StatusCode::NOT_FOUND, "ZENII_P4_IDENTITY_NOT_FOUND")
            }
            ZeniiError::Skill(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_P4_SKILL"),
            ZeniiError::SkillNotFound(_) => (StatusCode::NOT_FOUND, "ZENII_P4_SKILL_NOT_FOUND"),
            ZeniiError::User(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_P4_USER"),
            ZeniiError::Yaml(_) => (StatusCode::BAD_REQUEST, "ZENII_YAML_PARSE_ERROR"),
            ZeniiError::Validation(_) => (StatusCode::BAD_REQUEST, "ZENII_VALIDATION"),
            ZeniiError::Scheduler(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_SCHEDULER_ERROR")
            }
            ZeniiError::Plugin(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_PLUGIN_ERROR"),
            ZeniiError::PluginNotFound(_) => (StatusCode::NOT_FOUND, "ZENII_PLUGIN_NOT_FOUND"),
            ZeniiError::Workflow(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_WORKFLOW_ERROR")
            }
            ZeniiError::ModelCapability(_) => (StatusCode::BAD_REQUEST, "ZENII_MODEL_CAPABILITY"),
            ZeniiError::Other(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ZENII_INTERNAL_ERROR"),
        };

        // Sanitize internal error messages to prevent info leakage.
        // Log the detailed error server-side, return generic message to client.
        let message = match &self {
            ZeniiError::Sqlite(_) | ZeniiError::Database(_) | ZeniiError::Io(_) => {
                error!("Internal error ({}): {}", error_code, self);
                "Internal server error".to_string()
            }
            _ => self.to_string(),
        };

        let body = ErrorResponse {
            error_code: error_code.to_string(),
            message,
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use std::collections::HashSet;

    /// Helper to extract status, error_code, and message from a ZeniiError response.
    fn response_parts_full(err: ZeniiError) -> (StatusCode, String, String) {
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
        )
    }

    fn response_parts(err: ZeniiError) -> (StatusCode, String) {
        let (status, code, _) = response_parts_full(err);
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
        // Collect all error codes by creating one of each variant
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
            ZeniiError::ModelCapability("t".into()),
            ZeniiError::Other("t".into()),
        ];

        let mut codes = HashSet::new();
        for err in errors {
            let (_, code) = response_parts(err);
            assert!(codes.insert(code.clone()), "duplicate error code: {code}");
        }

        // 32 variants tested (Http skipped because reqwest::Error can't be easily constructed)
        assert_eq!(codes.len(), 32);
    }

    #[test]
    fn identity_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::Identity("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_P4_IDENTITY");
    }

    #[test]
    fn identity_not_found_maps_to_404() {
        let (status, code) = response_parts(ZeniiError::IdentityNotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "ZENII_P4_IDENTITY_NOT_FOUND");
    }

    #[test]
    fn skill_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::Skill("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_P4_SKILL");
    }

    #[test]
    fn skill_not_found_maps_to_404() {
        let (status, code) = response_parts(ZeniiError::SkillNotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "ZENII_P4_SKILL_NOT_FOUND");
    }

    #[test]
    fn user_error_maps_to_500() {
        let (status, code) = response_parts(ZeniiError::User("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "ZENII_P4_USER");
    }

    #[test]
    fn yaml_error_maps_to_400() {
        let yaml_err = serde_yaml::from_str::<serde_yaml::Value>(": bad").unwrap_err();
        let (status, code) = response_parts(ZeniiError::Yaml(yaml_err));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(code, "ZENII_YAML_PARSE_ERROR");
    }

    // --- WS-4.5: Error message sanitization ---

    #[test]
    fn sqlite_error_does_not_leak_path() {
        let err = ZeniiError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("/home/user/.local/share/zenii/data.db: disk I/O error".into()),
        ));
        let (status, code, message) = response_parts_full(err);
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
        let (_, _, message) = response_parts_full(err);
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
        let (_, _, message) = response_parts_full(err);
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
        let (_, _, message) = response_parts_full(err);
        assert!(
            message.contains("abc-123"),
            "NotFound should still include details for debugging"
        );
    }
}
