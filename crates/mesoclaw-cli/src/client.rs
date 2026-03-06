use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct MesoClient {
    base_url: String,
    token: Option<String>,
    http: Client,
}

impl MesoClient {
    pub fn new(host: &str, port: u16, token: Option<String>) -> Self {
        Self {
            base_url: format!("http://{host}:{port}"),
            token,
            http: Client::new(),
        }
    }

    pub fn ws_url(&self, path: &str) -> String {
        let base = self
            .base_url
            .replacen("http://", "ws://", 1)
            .replacen("https://", "wss://", 1);
        format!("{base}{path}")
    }

    pub fn auth_header_value(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
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

    pub async fn post_no_response<B: Serialize>(&self, path: &str, body: &B) -> Result<(), String> {
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
        Ok(())
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

    pub async fn delete(&self, path: &str) -> Result<(), String> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs_base_url() {
        let client = MesoClient::new("127.0.0.1", 18981, None);
        assert_eq!(client.base_url, "http://127.0.0.1:18981");

        let client = MesoClient::new("localhost", 9090, Some("tok".into()));
        assert_eq!(client.base_url, "http://localhost:9090");
        assert_eq!(client.auth_header_value(), Some("Bearer tok".to_string()));
    }

    #[test]
    fn client_ws_url() {
        let client = MesoClient::new("127.0.0.1", 18981, None);
        assert_eq!(client.ws_url("/ws/chat"), "ws://127.0.0.1:18981/ws/chat");
    }
}
