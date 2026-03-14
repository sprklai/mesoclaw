use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct ZeniiClient {
    base_url: String,
    ws_url: String,
    token: Option<String>,
    http: Client,
}

impl ZeniiClient {
    pub fn new(host: &str, port: u16, token: Option<String>) -> Self {
        Self {
            base_url: format!("http://{host}:{port}"),
            ws_url: format!("ws://{host}:{port}"),
            token,
            http: Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    pub fn auth_header_value(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }

    pub fn ws_chat_url(&self) -> String {
        format!("{}/ws/chat", self.ws_url)
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let mut req = self.http.get(format!("{}{path}", self.base_url));
        if let Some(ref val) = self.auth_header_value() {
            req = req.header("authorization", val);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let mut req = self
            .http
            .post(format!("{}{path}", self.base_url))
            .json(body);
        if let Some(ref val) = self.auth_header_value() {
            req = req.header("authorization", val);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let mut req = self.http.put(format!("{}{path}", self.base_url)).json(body);
        if let Some(ref val) = self.auth_header_value() {
            req = req.header("authorization", val);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    async fn delete_req(&self, path: &str) -> Result<(), String> {
        let mut req = self.http.delete(format!("{}{path}", self.base_url));
        if let Some(ref val) = self.auth_header_value() {
            req = req.header("authorization", val);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        Ok(())
    }

    pub async fn health(&self) -> Result<bool, String> {
        let resp: serde_json::Value = self.get("/health").await?;
        Ok(resp.get("status").and_then(|v| v.as_str()) == Some("ok"))
    }

    pub async fn list_sessions(&self) -> Result<Vec<serde_json::Value>, String> {
        self.get("/sessions").await
    }

    pub async fn create_session(&self) -> Result<serde_json::Value, String> {
        self.post("/sessions", &serde_json::json!({})).await
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), String> {
        self.delete_req(&format!("/sessions/{id}")).await
    }

    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<serde_json::Value>, String> {
        self.get(&format!("/sessions/{session_id}/messages")).await
    }

    pub async fn get_default_model(&self) -> Result<String, String> {
        let resp: serde_json::Value = self.get("/providers/default-model").await?;
        Ok(resp
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    pub async fn send_user_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> Result<serde_json::Value, String> {
        self.post(
            &format!("/sessions/{session_id}/messages"),
            &serde_json::json!({ "role": "user", "content": content }),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_base_url() {
        let client = ZeniiClient::new("127.0.0.1", 18981, None);
        assert_eq!(client.base_url(), "http://127.0.0.1:18981");

        let client = ZeniiClient::new("localhost", 9090, Some("tok".into()));
        assert_eq!(client.base_url(), "http://localhost:9090");
    }

    #[test]
    fn client_ws_url() {
        let client = ZeniiClient::new("127.0.0.1", 18981, None);
        assert_eq!(client.ws_url(), "ws://127.0.0.1:18981");
        assert_eq!(client.ws_chat_url(), "ws://127.0.0.1:18981/ws/chat");
    }

    #[test]
    fn client_auth_header() {
        let client = ZeniiClient::new("127.0.0.1", 18981, Some("my-token".into()));
        assert_eq!(
            client.auth_header_value(),
            Some("Bearer my-token".to_string())
        );
    }

    #[test]
    fn client_no_auth_header() {
        let client = ZeniiClient::new("127.0.0.1", 18981, None);
        assert_eq!(client.auth_header_value(), None);
    }
}
