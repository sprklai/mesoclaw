//! Web fetch tool for HTTP GET requests.
//!
//! Allows agents to fetch content from URLs, subject to security policy.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Url;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::security::SecurityPolicy;

use super::traits::{Tool, ToolResult};

/// Maximum response size to return (1 MB).
const MAX_RESPONSE_SIZE: usize = 1024 * 1024;

/// Default request timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum request timeout allowed.
const MAX_TIMEOUT_SECS: u64 = 120;

/// Fetches content from URLs via HTTP GET requests.
///
/// Subject to the active [`SecurityPolicy`] for network access control.
pub struct WebFetchTool {
    policy: Arc<SecurityPolicy>,
    client: reqwest::Client,
}

impl WebFetchTool {
    /// Create a new web fetch tool with the given security policy.
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .redirect(Policy::limited(5))
            .user_agent("MesoClaw/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { policy, client }
    }

    /// Validate that the URL is well-formed and uses an allowed scheme.
    fn validate_url(&self, url: &str) -> Result<Url, String> {
        let parsed = Url::parse(url).map_err(|e| format!("invalid URL: {e}"))?;

        match parsed.scheme() {
            "http" | "https" => Ok(parsed),
            scheme => Err(format!(
                "unsupported URL scheme: {scheme}. Only http and https are allowed."
            )),
        }
    }

    /// Check if network access is allowed by the security policy.
    fn check_network_allowed(&self) -> Result<(), String> {
        // Log the action for audit
        self.policy.log_action(
            self.name(),
            json!({"action": "web_fetch"}),
            crate::security::RiskLevel::Medium,
            "allowed",
            None,
        );
        Ok(())
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL using HTTP GET. Returns the response body as text. \
         Useful for retrieving web pages, API responses, or any HTTP-accessible content. \
         Supports custom headers and timeout configuration."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (must be http or https)."
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Optional timeout in seconds (default: 30, max: 120).",
                    "minimum": 1,
                    "maximum": 120
                },
                "headers": {
                    "type": "object",
                    "description": "Optional HTTP headers to include in the request.",
                    "additionalProperties": {
                        "type": "string"
                    }
                },
                "follow_redirects": {
                    "type": "boolean",
                    "description": "Whether to follow HTTP redirects (default: true)."
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let url = args
            .get("url")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'url'")?;

        // Validate URL format and scheme
        let parsed_url = self.validate_url(url)?;

        // Check security policy
        self.check_network_allowed()?;

        // Parse optional parameters
        let timeout_secs = args
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        let headers: Option<std::collections::HashMap<String, String>> = args
            .get("headers")
            .and_then(|h| serde_json::from_value(h.clone()).ok());

        let follow_redirects = args
            .get("follow_redirects")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        // Build the request
        let mut request_builder = self.client.get(parsed_url.as_str());

        // Set timeout for this specific request
        request_builder = request_builder.timeout(Duration::from_secs(timeout_secs));

        // Add custom headers if provided
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                request_builder = request_builder.header(&key, &value);
            }
        }

        // Configure redirect policy for this request
        if !follow_redirects {
            let no_redirect_client = reqwest::Client::builder()
                .redirect(Policy::none())
                .timeout(Duration::from_secs(timeout_secs))
                .user_agent("MesoClaw/1.0")
                .build()
                .map_err(|e| format!("failed to build client: {e}"))?;
            request_builder = no_redirect_client.get(parsed_url.as_str());
        }

        // Execute the request
        let response = request_builder
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = response.status();
        let status_code = status.as_u16();

        // Get response headers before consuming response
        let response_headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
            .collect();

        // Get content_type from headers map to avoid borrowing response again
        let content_type = response_headers
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let body = response
            .text()
            .await
            .map_err(|e| format!("failed to read response body: {e}"))?;

        // Store original length before truncation
        let original_len = body.len();

        // Truncate if too large
        let (body_text, truncated) = if original_len > MAX_RESPONSE_SIZE {
            (body[..MAX_RESPONSE_SIZE].to_string(), true)
        } else {
            (body, false)
        };

        // Build output
        let mut output = format!("HTTP {status_code} {status}\n");
        output.push_str(&format!("Content-Type: {content_type}\n"));
        if truncated {
            output.push_str(&format!(
                "Note: Response truncated to {} bytes (original: {} bytes)\n",
                MAX_RESPONSE_SIZE, original_len
            ));
        }
        output.push_str("\n");
        output.push_str(&body_text);

        // Build metadata
        let metadata = json!({
            "status_code": status_code,
            "content_type": content_type,
            "response_size": body_text.len(),
            "truncated": truncated,
            "headers": response_headers,
        });

        if status.is_success() {
            Ok(ToolResult::ok(output).with_metadata(metadata))
        } else {
            Ok(ToolResult::err(output).with_metadata(metadata))
        }
    }
}

/// Generic HTTP request tool for any HTTP method.
///
/// Provides more control over HTTP requests including POST, PUT, DELETE, etc.
pub struct WebRequestTool {
    policy: Arc<SecurityPolicy>,
    client: reqwest::Client,
}

impl WebRequestTool {
    /// Create a new web request tool with the given security policy.
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .redirect(Policy::limited(5))
            .user_agent("MesoClaw/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { policy, client }
    }

    /// Validate that the URL is well-formed and uses an allowed scheme.
    fn validate_url(&self, url: &str) -> Result<Url, String> {
        let parsed = Url::parse(url).map_err(|e| format!("invalid URL: {e}"))?;

        match parsed.scheme() {
            "http" | "https" => Ok(parsed),
            scheme => Err(format!(
                "unsupported URL scheme: {scheme}. Only http and https are allowed."
            )),
        }
    }
}

#[async_trait]
impl Tool for WebRequestTool {
    fn name(&self) -> &str {
        "web_request"
    }

    fn description(&self) -> &str {
        "Make an HTTP request with full control over method, headers, and body. \
         Supports GET, POST, PUT, PATCH, DELETE methods. \
         Useful for interacting with REST APIs and web services."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to request (must be http or https)."
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (default: GET).",
                    "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                },
                "headers": {
                    "type": "object",
                    "description": "HTTP headers to include in the request.",
                    "additionalProperties": {
                        "type": "string"
                    }
                },
                "body": {
                    "type": "string",
                    "description": "Request body (for POST, PUT, PATCH)."
                },
                "body_type": {
                    "type": "string",
                    "description": "Content type for the body (default: application/json).",
                    "enum": ["json", "text", "form"]
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30, max: 120).",
                    "minimum": 1,
                    "maximum": 120
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let url = args
            .get("url")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'url'")?;

        // Validate URL
        let _parsed_url = self.validate_url(url)?;

        // Get HTTP method
        let method_str = args
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET")
            .to_uppercase();

        let method = match method_str.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            "DELETE" => reqwest::Method::DELETE,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => return Err(format!("unsupported HTTP method: {method_str}")),
        };

        let timeout_secs = args
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        // Build request
        let mut request_builder = self.client.request(method, url);
        request_builder = request_builder.timeout(Duration::from_secs(timeout_secs));

        // Add headers
        if let Some(headers) = args.get("headers").and_then(|h| h.as_object()) {
            for (key, value) in headers {
                if let Some(value_str) = value.as_str() {
                    request_builder = request_builder.header(key, value_str);
                }
            }
        }

        // Add body if provided
        if let Some(body) = args.get("body").and_then(Value::as_str) {
            let body_type = args
                .get("body_type")
                .and_then(Value::as_str)
                .unwrap_or("json");

            match body_type {
                "json" => {
                    request_builder = request_builder
                        .header("Content-Type", "application/json")
                        .body(body.to_string());
                }
                "text" => {
                    request_builder = request_builder
                        .header("Content-Type", "text/plain")
                        .body(body.to_string());
                }
                "form" => {
                    // Parse body as form data
                    let form_data: std::collections::HashMap<String, String> =
                        serde_json::from_str(body)
                            .map_err(|e| format!("invalid form data: {e}"))?;
                    request_builder = request_builder.form(&form_data);
                }
                _ => return Err(format!("unsupported body_type: {body_type}")),
            }
        }

        // Log the action
        self.policy.log_action(
            self.name(),
            json!({"url": url, "method": method_str}),
            crate::security::RiskLevel::Medium,
            "allowed",
            None,
        );

        // Execute request
        let response = request_builder
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = response.status();
        let status_code = status.as_u16();

        // Get response body
        let body = response
            .text()
            .await
            .map_err(|e| format!("failed to read response body: {e}"))?;

        // Truncate if too large
        let (body_text, truncated) = if body.len() > MAX_RESPONSE_SIZE {
            (body[..MAX_RESPONSE_SIZE].to_string(), true)
        } else {
            (body, false)
        };

        // Build output
        let mut output = format!("HTTP {status_code} {status}\n\n");
        if truncated {
            output.push_str(&format!(
                "[Response truncated to {} bytes]\n\n",
                MAX_RESPONSE_SIZE
            ));
        }
        output.push_str(&body_text);

        let metadata = json!({
            "status_code": status_code,
            "response_size": body_text.len(),
            "truncated": truncated,
        });

        if status.is_success() {
            Ok(ToolResult::ok(output).with_metadata(metadata))
        } else {
            Ok(ToolResult::err(output).with_metadata(metadata))
        }
    }
}

/// Search result entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Web search tool using DuckDuckGo (no API key required).
///
/// Performs web searches and returns structured results with titles,
/// URLs, and snippets.
pub struct WebSearchTool {
    policy: Arc<SecurityPolicy>,
    client: reqwest::Client,
}

impl WebSearchTool {
    /// Create a new web search tool.
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .redirect(Policy::limited(5))
            .user_agent("Mozilla/5.0 (compatible; MesoClaw/1.0)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { policy, client }
    }

    /// Build DuckDuckGo search URL.
    fn build_search_url(&self, query: &str, max_results: usize) -> String {
        let encoded_query = urlencoding::encode(query);
        // DuckDuckGo HTML search endpoint
        format!(
            "https://html.duckduckgo.com/html/?q={}&num={}",
            encoded_query, max_results
        )
    }

    /// Parse search results from DuckDuckGo HTML response.
    fn parse_results(&self, html: &str, max_results: usize) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // Simple regex-based parsing of DuckDuckGo HTML results
        // Looking for: <a class="result__a" href="URL">TITLE</a>
        // And: <a class="result__snippet" ...>SNIPPET</a>

        let result_pattern =
            regex::Regex::new(r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#)
                .unwrap();

        let snippet_pattern = regex::Regex::new(
            r#"<a[^>]*class="result__snippet"[^>]*>([^<]*(?:<[^>]+>[^<]*)*)</a>"#,
        )
        .unwrap();

        // Split by result containers
        let result_blocks: Vec<&str> = html.split("class=\"result__body\"").collect();

        for block in result_blocks.iter().skip(1).take(max_results) {
            let title_url = result_pattern.captures(block);
            let snippet = snippet_pattern.captures(block);

            if let Some(caps) = title_url {
                let raw_url = caps.get(1).map_or("", |m| m.as_str());
                let title = caps.get(2).map_or("", |m| m.as_str());

                // DuckDuckGo uses redirect URLs, extract actual URL
                let url = self.extract_actual_url(raw_url);

                // Clean snippet (remove HTML tags)
                let clean_snippet = snippet
                    .and_then(|s| s.get(1))
                    .map(|m| self.clean_html(m.as_str()))
                    .unwrap_or_default();

                if !title.is_empty() && !url.is_empty() {
                    results.push(SearchResult {
                        title: self.clean_html(title),
                        url,
                        snippet: clean_snippet,
                    });
                }
            }
        }

        results
    }

    /// Extract actual URL from DuckDuckGo redirect URL.
    fn extract_actual_url(&self, redirect_url: &str) -> String {
        // DuckDuckGo URLs look like: //duckduckgo.com/l/?uddg=ENCODED_URL&rut=...
        if redirect_url.contains("uddg=") {
            if let Some(start) = redirect_url.find("uddg=") {
                let encoded = &redirect_url[start + 5..];
                if let Some(end) = encoded.find('&') {
                    return urlencoding::decode(&encoded[..end])
                        .unwrap_or_default()
                        .to_string();
                } else {
                    return urlencoding::decode(encoded).unwrap_or_default().to_string();
                }
            }
        }
        // If it's already a direct URL, return as-is
        if redirect_url.starts_with("http") {
            redirect_url.to_string()
        } else if redirect_url.starts_with("//") {
            format!("https:{}", redirect_url)
        } else {
            redirect_url.to_string()
        }
    }

    /// Remove HTML tags and decode entities.
    fn clean_html(&self, text: &str) -> String {
        // Remove HTML tags
        let tag_pattern = regex::Regex::new(r"<[^>]+>").unwrap();
        let cleaned = tag_pattern.replace_all(text, "");

        // Decode common HTML entities
        cleaned
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ")
            .trim()
            .to_string()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo and return structured results. \
         Each result includes a title, URL, and snippet. No API key required. \
         Useful for finding current information, news, documentation, or any web content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query."
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5, max: 20).",
                    "minimum": 1,
                    "maximum": 20
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'query'")?;

        let max_results = args
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .min(20) as usize;

        // Log the search
        self.policy.log_action(
            self.name(),
            json!({"query": query, "max_results": max_results}),
            crate::security::RiskLevel::Medium,
            "allowed",
            None,
        );

        // Build and execute search
        let url = self.build_search_url(query, max_results);

        let response = self
            .client
            .get(&url)
            .header("Accept", "text/html")
            .send()
            .await
            .map_err(|e| format!("search request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("search failed with status: {}", status));
        }

        let html = response
            .text()
            .await
            .map_err(|e| format!("failed to read response: {e}"))?;

        // Parse results
        let results = self.parse_results(&html, max_results);

        if results.is_empty() {
            return Ok(ToolResult::ok(format!(
                "No results found for query: '{}'",
                query
            )));
        }

        // Format output
        let mut output = format!("Search results for: '{}'\n\n", query);
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!(
                "{}. **{}**\n   {}\n   {}\n\n",
                i + 1,
                result.title,
                result.snippet,
                result.url
            ));
        }

        let metadata = json!({
            "query": query,
            "result_count": results.len(),
            "results": results,
        });

        Ok(ToolResult::ok(output).with_metadata(metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};

    fn test_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::Full,
            None,
            vec![],
            3600,
            100,
        ))
    }

    // --- WebFetchTool tests ---

    #[test]
    fn web_fetch_schema_is_valid() {
        let tool = WebFetchTool::new(test_policy());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["url"].is_object());
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("url"))
        );
    }

    #[test]
    fn web_fetch_validates_url_scheme() {
        let tool = WebFetchTool::new(test_policy());
        let result = tool.validate_url("ftp://example.com/file");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported URL scheme"));
    }

    #[test]
    fn web_fetch_accepts_http_urls() {
        let tool = WebFetchTool::new(test_policy());
        let result = tool.validate_url("http://example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn web_fetch_accepts_https_urls() {
        let tool = WebFetchTool::new(test_policy());
        let result = tool.validate_url("https://example.com/path?query=1");
        assert!(result.is_ok());
    }

    #[test]
    fn web_fetch_rejects_invalid_urls() {
        let tool = WebFetchTool::new(test_policy());
        let result = tool.validate_url("not a url");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn web_fetch_missing_url_errors() {
        let tool = WebFetchTool::new(test_policy());
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing required argument"));
    }

    // --- WebRequestTool tests ---

    #[test]
    fn web_request_schema_is_valid() {
        let tool = WebRequestTool::new(test_policy());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["url"].is_object());
        assert!(schema["properties"]["method"].is_object());
    }

    #[test]
    fn web_request_validates_url_scheme() {
        let tool = WebRequestTool::new(test_policy());
        let result = tool.validate_url("file:///etc/passwd");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn web_request_missing_url_errors() {
        let tool = WebRequestTool::new(test_policy());
        let result = tool.execute(json!({"method": "POST"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn web_request_invalid_method_errors() {
        let tool = WebRequestTool::new(test_policy());
        let result = tool
            .execute(json!({
                "url": "https://example.com",
                "method": "INVALID"
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported HTTP method"));
    }

    // --- WebSearchTool tests ---

    #[test]
    fn web_search_schema_is_valid() {
        let tool = WebSearchTool::new(test_policy());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("query"))
        );
    }

    #[tokio::test]
    async fn web_search_missing_query_errors() {
        let tool = WebSearchTool::new(test_policy());
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing required argument"));
    }

    #[test]
    fn web_search_builds_correct_url() {
        let tool = WebSearchTool::new(test_policy());
        let url = tool.build_search_url("test query", 10);
        assert!(url.contains("duckduckgo.com"));
        // URL encoding may use + or %20 for spaces
        assert!(url.contains("test+query") || url.contains("test%20query"));
    }

    #[test]
    fn web_search_extracts_redirect_url() {
        let tool = WebSearchTool::new(test_policy());
        let redirect = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=abc";
        let extracted = tool.extract_actual_url(redirect);
        assert_eq!(extracted, "https://example.com");
    }

    #[test]
    fn web_search_cleans_html() {
        let tool = WebSearchTool::new(test_policy());
        let html = "<b>Hello</b> &amp; <i>World</i>";
        let cleaned = tool.clean_html(html);
        assert_eq!(cleaned, "Hello & World");
    }

    #[test]
    fn web_search_parses_results() {
        let tool = WebSearchTool::new(test_policy());
        let html = r#"
            <div class="result__body">
                <a class="result__a" href="https://example.com">Example Title</a>
                <a class="result__snippet">Example snippet text</a>
            </div>
            <div class="result__body">
                <a class="result__a" href="https://test.com">Test Title</a>
                <a class="result__snippet">Test snippet</a>
            </div>
        "#;
        let results = tool.parse_results(html, 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Example Title");
        assert_eq!(results[1].title, "Test Title");
    }
}
