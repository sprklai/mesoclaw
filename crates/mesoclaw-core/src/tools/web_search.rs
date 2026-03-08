use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info, warn};
use websearch::SearchOptions;

use crate::credential::CredentialStore;
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

/// Search provider used in the cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchProvider {
    Tavily,
    Brave,
    DuckDuckGo,
}

impl std::fmt::Display for SearchProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tavily => write!(f, "Tavily"),
            Self::Brave => write!(f, "Brave"),
            Self::DuckDuckGo => write!(f, "DuckDuckGo"),
        }
    }
}

/// Web search tool with provider cascade: Tavily -> Brave -> DuckDuckGo.
///
/// Uses credential store to resolve API keys at runtime. Falls back to
/// DuckDuckGo (no key required) when no paid provider keys are available.
/// Backed by the `websearch` crate for all provider implementations.
pub struct WebSearchTool {
    credentials: Arc<dyn CredentialStore>,
    timeout_secs: u64,
    max_results: usize,
}

impl WebSearchTool {
    pub fn new(
        credentials: Arc<dyn CredentialStore>,
        timeout_secs: u64,
        max_results: usize,
    ) -> Self {
        Self {
            credentials,
            timeout_secs,
            max_results,
        }
    }

    /// Select the best available search provider based on stored credentials.
    async fn select_provider(&self) -> (SearchProvider, Option<String>) {
        if let Ok(Some(key)) = self.credentials.get("api_key:tavily").await
            && !key.is_empty()
        {
            return (SearchProvider::Tavily, Some(key));
        }
        if let Ok(Some(key)) = self.credentials.get("api_key:brave").await
            && !key.is_empty()
        {
            return (SearchProvider::Brave, Some(key));
        }
        (SearchProvider::DuckDuckGo, None)
    }

    /// Execute search using the `websearch` crate with the selected provider.
    async fn do_search(
        &self,
        query: &str,
        num_results: usize,
        provider: SearchProvider,
        api_key: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        let options = match provider {
            SearchProvider::Tavily => {
                let key = api_key.unwrap_or("");
                let tavily = websearch::providers::tavily::TavilyProvider::new(key)
                    .map_err(|e| MesoError::Tool(format!("Tavily provider init failed: {e}")))?;
                SearchOptions {
                    query: query.to_string(),
                    provider: Box::new(tavily),
                    max_results: Some(num_results as u32),
                    timeout: Some(self.timeout_secs * 1000),
                    ..Default::default()
                }
            }
            SearchProvider::Brave => {
                let key = api_key.unwrap_or("");
                let brave = websearch::providers::brave::BraveProvider::new(key)
                    .map_err(|e| MesoError::Tool(format!("Brave provider init failed: {e}")))?;
                SearchOptions {
                    query: query.to_string(),
                    provider: Box::new(brave),
                    max_results: Some(num_results as u32),
                    timeout: Some(self.timeout_secs * 1000),
                    ..Default::default()
                }
            }
            SearchProvider::DuckDuckGo => {
                let ddg = websearch::providers::duckduckgo::DuckDuckGoProvider::new();
                SearchOptions {
                    query: query.to_string(),
                    provider: Box::new(ddg),
                    max_results: Some(num_results as u32),
                    timeout: Some(self.timeout_secs * 1000),
                    ..Default::default()
                }
            }
        };

        let results = websearch::web_search(options)
            .await
            .map_err(|e| MesoError::Tool(format!("{provider} search failed: {e}")))?;

        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .take(num_results)
            .map(|r| {
                serde_json::json!({
                    "title": r.title,
                    "url": r.url,
                    "snippet": r.snippet.unwrap_or_default(),
                })
            })
            .collect();

        Ok(json_results)
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using Tavily, Brave, or DuckDuckGo (automatic provider cascade)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "num_results": { "type": "integer", "description": "Number of results (max 20)", "default": 5 }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'query' argument".into()))?;

        if query.trim().is_empty() {
            return Err(MesoError::Tool("query must not be empty".into()));
        }

        let num_results = args
            .get("num_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(5)
            .min(self.max_results);

        let (provider, api_key) = self.select_provider().await;
        info!(provider = %provider, query = %query, "Performing web search");

        let results = self
            .do_search(query, num_results, provider, api_key.as_deref())
            .await;

        match results {
            Ok(results) if results.is_empty() => {
                if provider != SearchProvider::DuckDuckGo {
                    warn!(provider = %provider, "No results, falling back to DuckDuckGo");
                    match self
                        .do_search(query, num_results, SearchProvider::DuckDuckGo, None)
                        .await
                    {
                        Ok(fallback) if !fallback.is_empty() => {
                            let json = serde_json::to_string_pretty(&fallback)
                                .unwrap_or_else(|_| "[]".to_string());
                            Ok(ToolResult::ok(json))
                        }
                        Ok(_) => Ok(ToolResult::ok("No search results found.")),
                        Err(e) => {
                            warn!("DuckDuckGo fallback also failed: {e}");
                            Ok(ToolResult::ok("No search results found."))
                        }
                    }
                } else {
                    Ok(ToolResult::ok("No search results found."))
                }
            }
            Ok(results) => {
                debug!(count = results.len(), provider = %provider, "Search results received");
                let json =
                    serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string());
                Ok(ToolResult::ok(json))
            }
            Err(e) => {
                if provider != SearchProvider::DuckDuckGo {
                    warn!(provider = %provider, error = %e, "Provider failed, falling back to DuckDuckGo");
                    match self
                        .do_search(query, num_results, SearchProvider::DuckDuckGo, None)
                        .await
                    {
                        Ok(fallback) if !fallback.is_empty() => {
                            let json = serde_json::to_string_pretty(&fallback)
                                .unwrap_or_else(|_| "[]".to_string());
                            Ok(ToolResult::ok(json))
                        }
                        Ok(_) => Ok(ToolResult::ok("No search results found.")),
                        Err(fallback_err) => Err(MesoError::Tool(format!(
                            "{provider} failed: {e}; DuckDuckGo fallback also failed: {fallback_err}"
                        ))),
                    }
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::InMemoryCredentialStore;

    fn mock_credentials() -> Arc<dyn CredentialStore> {
        Arc::new(InMemoryCredentialStore::new())
    }

    fn tool_with_creds(creds: Arc<dyn CredentialStore>) -> WebSearchTool {
        WebSearchTool::new(creds, 30, 20)
    }

    // WS.1
    #[test]
    fn schema_is_valid() {
        let tool = tool_with_creds(mock_credentials());
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["num_results"].is_object());
    }

    // WS.2
    #[tokio::test]
    async fn missing_query_errors() {
        let tool = tool_with_creds(mock_credentials());
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    // WS.3
    #[test]
    fn name_is_web_search() {
        let tool = tool_with_creds(mock_credentials());
        assert_eq!(tool.name(), "web_search");
    }

    // WS.4
    #[tokio::test]
    async fn cascade_prefers_tavily() {
        let creds = Arc::new(InMemoryCredentialStore::new());
        creds.set("api_key:tavily", "tvly-test123").await.unwrap();
        creds.set("api_key:brave", "brave-test123").await.unwrap();
        let tool = tool_with_creds(creds.clone() as Arc<dyn CredentialStore>);
        let (provider, key) = tool.select_provider().await;
        assert_eq!(provider, SearchProvider::Tavily);
        assert_eq!(key.unwrap(), "tvly-test123");
    }

    // WS.5
    #[tokio::test]
    async fn cascade_falls_to_brave() {
        let creds = Arc::new(InMemoryCredentialStore::new());
        creds.set("api_key:brave", "brave-test123").await.unwrap();
        let tool = tool_with_creds(creds.clone() as Arc<dyn CredentialStore>);
        let (provider, key) = tool.select_provider().await;
        assert_eq!(provider, SearchProvider::Brave);
        assert_eq!(key.unwrap(), "brave-test123");
    }

    // WS.6
    #[tokio::test]
    async fn cascade_falls_to_duckduckgo() {
        let tool = tool_with_creds(mock_credentials());
        let (provider, key) = tool.select_provider().await;
        assert_eq!(provider, SearchProvider::DuckDuckGo);
        assert!(key.is_none());
    }

    // WS.7
    #[tokio::test]
    async fn max_results_capped() {
        let tool = WebSearchTool::new(mock_credentials(), 30, 3);
        // num_results=10 should be capped to max_results=3
        let args = serde_json::json!({"query": "test", "num_results": 10});
        let num = args
            .get("num_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(5)
            .min(tool.max_results);
        assert_eq!(num, 3);
    }

    // WS.8
    #[tokio::test]
    async fn empty_query_rejected() {
        let tool = tool_with_creds(mock_credentials());
        let result = tool.execute(serde_json::json!({"query": ""})).await;
        assert!(result.is_err());
        let result = tool.execute(serde_json::json!({"query": "   "})).await;
        assert!(result.is_err());
    }

    // WS.9
    #[test]
    fn config_timeout_used() {
        let creds = mock_credentials();
        let tool = WebSearchTool::new(creds, 45, 10);
        assert_eq!(tool.timeout_secs, 45);
        assert_eq!(tool.max_results, 10);
    }

    // WS.10 — Integration test (requires network)
    #[tokio::test]
    #[ignore]
    async fn duckduckgo_fallback_returns_results() {
        let tool = tool_with_creds(mock_credentials());
        let result = tool
            .execute(serde_json::json!({"query": "Rust programming language", "num_results": 3}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(!result.output.contains("No search results"));
    }

    // WS.11 — Integration test (requires TAVILY_API_KEY env)
    #[tokio::test]
    #[ignore]
    async fn tavily_search_with_key() {
        let api_key = std::env::var("TAVILY_API_KEY").expect("TAVILY_API_KEY must be set");
        let creds = Arc::new(InMemoryCredentialStore::new());
        creds.set("api_key:tavily", &api_key).await.unwrap();
        let tool = tool_with_creds(creds as Arc<dyn CredentialStore>);
        let result = tool
            .execute(serde_json::json!({"query": "latest Rust release"}))
            .await
            .unwrap();
        assert!(result.success);
    }
}
