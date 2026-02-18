//! Newline-delimited JSON protocol for communicating with sidecar processes.
//!
//! Each request/response is a single JSON object terminated by `\n`.
//! The protocol is intentionally minimal — inspired by JSON-RPC 2.0:
//!
//! ```text
//! → {"id":"1","method":"execute","params":{...}}\n
//! ← {"id":"1","result":{...}}\n
//! ← {"id":"1","error":{"code":1,"message":"..."}}\n
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};

// ─── Wire types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<StdioError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioError {
    pub code: i32,
    pub message: String,
}

impl StdioResponse {
    /// Convert to a `Result`, treating `error` as the failure case.
    pub fn into_result(self) -> Result<Value, String> {
        if let Some(err) = self.error {
            Err(format!("[{}] {}", err.code, err.message))
        } else {
            Ok(self.result.unwrap_or(Value::Null))
        }
    }
}

// ─── I/O helpers ───────────────────────────────────────────────────────────────

/// Write a request as a single newline-terminated JSON line to the child stdin.
pub async fn send_request(stdin: &mut ChildStdin, request: &StdioRequest) -> Result<(), String> {
    let mut line = serde_json::to_string(request).map_err(|e| e.to_string())?;
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())
}

/// Read a single newline-terminated JSON line from the child stdout.
pub async fn read_response(reader: &mut BufReader<ChildStdout>) -> Result<StdioResponse, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| e.to_string())?;
    if line.is_empty() {
        return Err("sidecar closed stdout unexpectedly".to_string());
    }
    serde_json::from_str::<StdioResponse>(&line)
        .map_err(|e| format!("protocol parse error: {e} (raw: {line:?})"))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_to_json() {
        let req = StdioRequest {
            id: "42".to_string(),
            method: "execute".to_string(),
            params: json!({"input": "hello"}),
        };
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"id\":\"42\""));
        assert!(s.contains("\"method\":\"execute\""));
        assert!(s.contains("\"input\":\"hello\""));
    }

    #[test]
    fn response_with_result_deserializes() {
        let raw = r#"{"id":"1","result":{"output":"ok"}}"#;
        let resp: StdioResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.id, "1");
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn response_with_error_deserializes() {
        let raw = r#"{"id":"2","error":{"code":-1,"message":"not found"}}"#;
        let resp: StdioResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.id, "2");
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -1);
        assert_eq!(err.message, "not found");
    }

    #[test]
    fn into_result_ok() {
        let resp = StdioResponse {
            id: "1".to_string(),
            result: Some(json!({"x": 1})),
            error: None,
        };
        let r = resp.into_result();
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), json!({"x": 1}));
    }

    #[test]
    fn into_result_err() {
        let resp = StdioResponse {
            id: "2".to_string(),
            result: None,
            error: Some(StdioError {
                code: 42,
                message: "boom".to_string(),
            }),
        };
        let r = resp.into_result();
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("boom"));
    }

    #[test]
    fn response_null_result_is_ok() {
        let raw = r#"{"id":"3"}"#;
        let resp: StdioResponse = serde_json::from_str(raw).unwrap();
        let r = resp.into_result();
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), Value::Null);
    }
}
