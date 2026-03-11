use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tracing::error;

use crate::MesoError;

#[derive(Debug, Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
}

impl IntoResponse for MesoError {
    fn into_response(self) -> Response {
        let (status, error_code) = match &self {
            MesoError::NotFound(_) => (StatusCode::NOT_FOUND, "MESO_NOT_FOUND"),
            MesoError::PolicyDenied(_) => (StatusCode::FORBIDDEN, "MESO_POLICY_DENIED"),
            MesoError::RateLimited(_) => (StatusCode::TOO_MANY_REQUESTS, "MESO_RATE_LIMITED"),
            MesoError::Auth(_) => (StatusCode::UNAUTHORIZED, "MESO_AUTH_REQUIRED"),
            MesoError::Serialization(_) => (StatusCode::BAD_REQUEST, "MESO_BAD_REQUEST"),
            MesoError::TomlParse(_) => (StatusCode::BAD_REQUEST, "MESO_TOML_PARSE_ERROR"),
            MesoError::Config(_) => (StatusCode::UNPROCESSABLE_ENTITY, "MESO_CONFIG_ERROR"),
            MesoError::Database(_) => (StatusCode::SERVICE_UNAVAILABLE, "MESO_DB_ERROR"),
            MesoError::Sqlite(_) => (StatusCode::SERVICE_UNAVAILABLE, "MESO_SQLITE_ERROR"),
            MesoError::Agent(_) => (StatusCode::BAD_GATEWAY, "MESO_AGENT_ERROR"),
            MesoError::Http(_) => (StatusCode::BAD_GATEWAY, "MESO_HTTP_ERROR"),
            MesoError::Tool(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_TOOL_ERROR"),
            MesoError::Memory(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_MEMORY_ERROR"),
            MesoError::Embedding(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_EMBEDDING_ERROR"),
            MesoError::Credential(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "MESO_CREDENTIAL_ERROR")
            }
            MesoError::Gateway(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_GATEWAY_ERROR"),
            MesoError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_IO_ERROR"),
            MesoError::EventBus(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_EVENT_ERROR"),
            MesoError::Channel(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_CHANNEL_ERROR"),
            MesoError::Context(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_CONTEXT"),
            MesoError::TomlSerialize(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "MESO_TOML_SERIALIZE_ERROR",
            ),
            MesoError::Identity(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_P4_IDENTITY"),
            MesoError::IdentityNotFound(_) => (StatusCode::NOT_FOUND, "MESO_P4_IDENTITY_NOT_FOUND"),
            MesoError::Skill(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_P4_SKILL"),
            MesoError::SkillNotFound(_) => (StatusCode::NOT_FOUND, "MESO_P4_SKILL_NOT_FOUND"),
            MesoError::User(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_P4_USER"),
            MesoError::Yaml(_) => (StatusCode::BAD_REQUEST, "MESO_YAML_PARSE_ERROR"),
            MesoError::Validation(_) => (StatusCode::BAD_REQUEST, "MESO_VALIDATION"),
            MesoError::Scheduler(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_SCHEDULER_ERROR"),
            MesoError::Plugin(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_PLUGIN_ERROR"),
            MesoError::PluginNotFound(_) => (StatusCode::NOT_FOUND, "MESO_PLUGIN_NOT_FOUND"),
            MesoError::ModelCapability(_) => (StatusCode::BAD_REQUEST, "MESO_MODEL_CAPABILITY"),
            MesoError::Other(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MESO_INTERNAL_ERROR"),
        };

        // Sanitize internal error messages to prevent info leakage.
        // Log the detailed error server-side, return generic message to client.
        let message = match &self {
            MesoError::Sqlite(_) | MesoError::Database(_) | MesoError::Io(_) => {
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

    /// Helper to extract status, error_code, and message from a MesoError response.
    fn response_parts_full(err: MesoError) -> (StatusCode, String, String) {
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

    fn response_parts(err: MesoError) -> (StatusCode, String) {
        let (status, code, _) = response_parts_full(err);
        (status, code)
    }

    #[test]
    fn not_found_maps_to_404() {
        let (status, code) = response_parts(MesoError::NotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "MESO_NOT_FOUND");
    }

    #[test]
    fn policy_denied_maps_to_403() {
        let (status, code) = response_parts(MesoError::PolicyDenied("blocked".into()));
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(code, "MESO_POLICY_DENIED");
    }

    #[test]
    fn rate_limited_maps_to_429() {
        let (status, code) = response_parts(MesoError::RateLimited("slow down".into()));
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(code, "MESO_RATE_LIMITED");
    }

    #[test]
    fn auth_maps_to_401() {
        let (status, code) = response_parts(MesoError::Auth("no token".into()));
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(code, "MESO_AUTH_REQUIRED");
    }

    #[test]
    fn serialization_maps_to_400() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let (status, code) = response_parts(MesoError::Serialization(json_err));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(code, "MESO_BAD_REQUEST");
    }

    #[test]
    fn config_maps_to_422() {
        let (status, code) = response_parts(MesoError::Config("bad config".into()));
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(code, "MESO_CONFIG_ERROR");
    }

    #[test]
    fn database_maps_to_503() {
        let (status, code) = response_parts(MesoError::Database("db down".into()));
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "MESO_DB_ERROR");
    }

    #[test]
    fn agent_maps_to_502() {
        let (status, code) = response_parts(MesoError::Agent("agent failed".into()));
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(code, "MESO_AGENT_ERROR");
    }

    #[test]
    fn tool_maps_to_500() {
        let (status, code) = response_parts(MesoError::Tool("tool broke".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "MESO_TOOL_ERROR");
    }

    #[test]
    fn other_maps_to_500() {
        let (status, code) = response_parts(MesoError::Other("unknown".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "MESO_INTERNAL_ERROR");
    }

    #[test]
    fn gateway_maps_to_500() {
        let (status, code) = response_parts(MesoError::Gateway("gw error".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "MESO_GATEWAY_ERROR");
    }

    #[test]
    fn all_error_codes_unique() {
        // Collect all error codes by creating one of each variant
        let json_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
        let toml_err = toml::from_str::<toml::Value>("= bad").unwrap_err();
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");

        let errors: Vec<MesoError> = vec![
            MesoError::NotFound("t".into()),
            MesoError::PolicyDenied("t".into()),
            MesoError::RateLimited("t".into()),
            MesoError::Auth("t".into()),
            MesoError::Serialization(json_err),
            MesoError::TomlParse(toml_err),
            MesoError::Config("t".into()),
            MesoError::Database("t".into()),
            MesoError::Sqlite(rusqlite::Error::InvalidParameterName("t".into())),
            MesoError::Agent("t".into()),
            // Skip Http — requires a real reqwest::Error which is hard to construct
            MesoError::Tool("t".into()),
            MesoError::Memory("t".into()),
            MesoError::Embedding("t".into()),
            MesoError::Credential("t".into()),
            MesoError::Gateway("t".into()),
            MesoError::Io(io_err),
            MesoError::EventBus("t".into()),
            MesoError::Channel("t".into()),
            MesoError::Context("t".into()),
            {
                use serde::ser::Error as _;
                MesoError::TomlSerialize(toml::ser::Error::custom("t"))
            },
            MesoError::Identity("t".into()),
            MesoError::IdentityNotFound("t".into()),
            MesoError::Skill("t".into()),
            MesoError::SkillNotFound("t".into()),
            MesoError::User("t".into()),
            {
                let yaml_err =
                    serde_yaml::from_str::<serde_yaml::Value>(": bad: yaml:").unwrap_err();
                MesoError::Yaml(yaml_err)
            },
            MesoError::Validation("t".into()),
            MesoError::Scheduler("t".into()),
            MesoError::Plugin("t".into()),
            MesoError::PluginNotFound("t".into()),
            MesoError::ModelCapability("t".into()),
            MesoError::Other("t".into()),
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
        let (status, code) = response_parts(MesoError::Identity("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "MESO_P4_IDENTITY");
    }

    #[test]
    fn identity_not_found_maps_to_404() {
        let (status, code) = response_parts(MesoError::IdentityNotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "MESO_P4_IDENTITY_NOT_FOUND");
    }

    #[test]
    fn skill_maps_to_500() {
        let (status, code) = response_parts(MesoError::Skill("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "MESO_P4_SKILL");
    }

    #[test]
    fn skill_not_found_maps_to_404() {
        let (status, code) = response_parts(MesoError::SkillNotFound("missing".into()));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(code, "MESO_P4_SKILL_NOT_FOUND");
    }

    #[test]
    fn user_error_maps_to_500() {
        let (status, code) = response_parts(MesoError::User("broken".into()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "MESO_P4_USER");
    }

    #[test]
    fn yaml_error_maps_to_400() {
        let yaml_err = serde_yaml::from_str::<serde_yaml::Value>(": bad").unwrap_err();
        let (status, code) = response_parts(MesoError::Yaml(yaml_err));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(code, "MESO_YAML_PARSE_ERROR");
    }

    // --- WS-4.5: Error message sanitization ---

    #[test]
    fn sqlite_error_does_not_leak_path() {
        let err = MesoError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("/home/user/.local/share/mesoclaw/data.db: disk I/O error".into()),
        ));
        let (status, code, message) = response_parts_full(err);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "MESO_SQLITE_ERROR");
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
        let err =
            MesoError::Database("connection to /var/db/mesoclaw.db failed: SQLITE_BUSY".into());
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
        let err = MesoError::Io(std::io::Error::new(
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
        let err = MesoError::NotFound("session abc-123".into());
        let (_, _, message) = response_parts_full(err);
        assert!(
            message.contains("abc-123"),
            "NotFound should still include details for debugging"
        );
    }
}
