use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex;

use crate::tools::traits::{Tool, ToolResult};
use crate::wiki::WikiManager;
use crate::{Result, ZeniiError};

/// Agent tool for searching, retrieving, and listing wiki pages.
pub struct WikiSearchTool {
    wiki: Arc<Mutex<WikiManager>>,
}

impl WikiSearchTool {
    pub fn new(wiki: Arc<Mutex<WikiManager>>) -> Self {
        Self { wiki }
    }
}

#[async_trait]
impl Tool for WikiSearchTool {
    fn name(&self) -> &str {
        "wiki"
    }

    fn description(&self) -> &str {
        "Search, retrieve, list, or query pages from the knowledge wiki. Use 'search' to find pages by keyword, 'get' to fetch a specific page by slug, 'list' to browse all available pages with their TLDRs, or 'query' to retrieve the full body of the most relevant pages for a natural-language question (pipe the output into an LLM for synthesis)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["search", "get", "list", "query"],
                    "description": "The wiki operation to perform"
                },
                "query": {
                    "type": "string",
                    "description": "Keyword search query (required for 'search')"
                },
                "slug": {
                    "type": "string",
                    "description": "Page slug to retrieve (required for 'get')"
                },
                "question": {
                    "type": "string",
                    "description": "Natural-language question whose answer should be found in the wiki (required for 'query')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results for 'search', 'list', or 'query' (defaults: search=10, list=10, query=5)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("missing 'action' field".into()))?;

        match action {
            "search" => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'query' for search".into()))?
                    .to_string();
                let limit = args["limit"].as_u64().unwrap_or(10) as usize;

                let wiki = self.wiki.clone().lock_owned().await;
                let pages = tokio::task::spawn_blocking(move || {
                    wiki.search_pages(&query)
                        .map(|v| v.into_iter().take(limit).collect::<Vec<_>>())
                })
                .await
                .map_err(|e| ZeniiError::Tool(format!("wiki task panicked: {e}")))??;

                if pages.is_empty() {
                    return Ok(ToolResult::ok("No wiki pages found matching that query."));
                }

                let results: Vec<serde_json::Value> = pages
                    .into_iter()
                    // .take(limit) — already applied inside spawn_blocking
                    .map(|p| {
                        json!({
                            "slug": p.slug,
                            "title": p.title,
                            "type": p.page_type,
                            "tldr": p.tldr,
                            "tags": p.tags,
                        })
                    })
                    .collect();

                Ok(ToolResult::ok(
                    serde_json::to_string_pretty(&results).unwrap_or_default(),
                ))
            }
            "get" => {
                let slug = args["slug"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'slug' for get".into()))?
                    .to_string();

                let wiki = self.wiki.clone().lock_owned().await;
                let page = tokio::task::spawn_blocking(move || wiki.get_page(&slug))
                    .await
                    .map_err(|e| ZeniiError::Tool(format!("wiki task panicked: {e}")))??;

                match page {
                    Some(p) => {
                        let result = json!({
                            "slug": p.slug,
                            "title": p.title,
                            "type": p.page_type,
                            "tldr": p.tldr,
                            "body": p.body,
                            "tags": p.tags,
                            "related": p.related,
                            "updated": p.updated,
                        });
                        Ok(ToolResult::ok(
                            serde_json::to_string_pretty(&result).unwrap_or_default(),
                        ))
                    }
                    None => Ok(ToolResult::ok(format!(
                        "No wiki page found with slug '{}'.",
                        args["slug"].as_str().unwrap_or("")
                    ))),
                }
            }
            "list" => {
                let limit = args["limit"].as_u64().unwrap_or(10) as usize;

                let wiki = self.wiki.clone().lock_owned().await;
                let pages = tokio::task::spawn_blocking(move || {
                    wiki.list_pages()
                        .map(|v| v.into_iter().take(limit).collect::<Vec<_>>())
                })
                .await
                .map_err(|e| ZeniiError::Tool(format!("wiki task panicked: {e}")))??;

                if pages.is_empty() {
                    return Ok(ToolResult::ok(
                        "The wiki has no pages yet. Ingest a source to get started.",
                    ));
                }

                let results: Vec<serde_json::Value> = pages
                    .into_iter()
                    // .take(limit) — already applied inside spawn_blocking
                    .map(|p| {
                        json!({
                            "slug": p.slug,
                            "title": p.title,
                            "type": p.page_type,
                            "tldr": p.tldr,
                        })
                    })
                    .collect();

                Ok(ToolResult::ok(
                    serde_json::to_string_pretty(&results).unwrap_or_default(),
                ))
            }
            "query" => {
                let question = args["question"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'question' for query".into()))?
                    .to_string();
                let limit = args["limit"].as_u64().unwrap_or(5) as usize;

                let wiki = self.wiki.clone().lock_owned().await;
                let pages = tokio::task::spawn_blocking(move || {
                    wiki.search_pages(&question)
                        .map(|v| v.into_iter().take(limit).collect::<Vec<_>>())
                })
                .await
                .map_err(|e| ZeniiError::Tool(format!("wiki task panicked: {e}")))??;

                if pages.is_empty() {
                    return Ok(ToolResult::ok(
                        "No relevant wiki pages found for this question.",
                    ));
                }

                // Return full page bodies as context, suitable for piping into an LLM node.
                let context = pages
                    .into_iter()
                    .map(|p| format!("### {} ({})\n\n{}", p.title, p.slug, p.body))
                    .collect::<Vec<_>>()
                    .join("\n\n---\n\n");

                Ok(ToolResult::ok(context))
            }
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: search, get, list, query"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::TempDir;
    use tokio::sync::Mutex;

    use super::WikiSearchTool;
    use crate::tools::traits::Tool;
    use crate::wiki::WikiManager;

    const TEST_PAGE_CONTENT: &str = r#"---
title: "Self Attention"
type: concept
tags: [transformers, attention]
aliases: []
related: []
confidence: high
category: architecture
sources: []
updated: 2026-04-15
---

## TLDR
Self-attention lets each token attend to all other tokens in the sequence.

## Body
Used in transformer models.
"#;

    fn setup() -> (TempDir, WikiSearchTool) {
        let dir = TempDir::new().unwrap();
        let wiki = WikiManager::new(dir.path().to_path_buf()).unwrap();
        wiki.write_page("concepts", "self-attention", TEST_PAGE_CONTENT)
            .unwrap();
        let tool = WikiSearchTool::new(Arc::new(Mutex::new(wiki)));
        (dir, tool)
    }

    // WT.1 — search "attention" returns self-attention
    #[tokio::test]
    async fn wiki_tool_search_returns_match() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "search",
                "query": "attention"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("self-attention"));
    }

    // WT.2 — search with no match returns "No wiki pages found"
    #[tokio::test]
    async fn wiki_tool_search_no_match() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "search",
                "query": "quantum-computing-xyz"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("No wiki pages found"));
    }

    // WT.3 — get by slug returns page content
    #[tokio::test]
    async fn wiki_tool_get_returns_page() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "get",
                "slug": "self-attention"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Self Attention"));
        assert!(result.output.contains("transformer"));
    }

    // WT.4 — get unknown slug returns "No wiki page found"
    #[tokio::test]
    async fn wiki_tool_get_unknown_slug() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "get",
                "slug": "does-not-exist"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("No wiki page found"));
    }

    // WT.5 — list returns all pages
    #[tokio::test]
    async fn wiki_tool_list_returns_pages() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "list"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("self-attention"));
    }

    // WT.6 — unknown action returns error
    #[tokio::test]
    async fn wiki_tool_invalid_action() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "delete"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Unknown action"));
    }

    // WT.8 — search without query returns Validation error
    #[tokio::test]
    async fn wiki_tool_search_missing_query() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({ "action": "search" }))
            .await;
        assert!(matches!(result, Err(crate::ZeniiError::Validation(_))));
    }

    // WT.9 — get without slug returns Validation error
    #[tokio::test]
    async fn wiki_tool_get_missing_slug() {
        let (_dir, tool) = setup();
        let result = tool.execute(serde_json::json!({ "action": "get" })).await;
        assert!(matches!(result, Err(crate::ZeniiError::Validation(_))));
    }

    // WT.10 — query with matching question returns page body context
    #[tokio::test]
    async fn wiki_tool_query_returns_context() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "query",
                "question": "attention",
                "limit": 3
            }))
            .await
            .unwrap();

        assert!(result.success);
        // Output contains the heading format: ### {title} ({slug})
        assert!(result.output.contains("Self Attention"));
        assert!(result.output.contains("self-attention"));
    }

    // WT.11 — query with no match returns informative message
    #[tokio::test]
    async fn wiki_tool_query_no_match() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "action": "query",
                "question": "quantum-xyz-nonexistent"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("No relevant wiki pages found"));
    }

    // WT.12 — query without question returns Validation error
    #[tokio::test]
    async fn wiki_tool_query_missing_question() {
        let (_dir, tool) = setup();
        let result = tool.execute(serde_json::json!({ "action": "query" })).await;
        assert!(matches!(result, Err(crate::ZeniiError::Validation(_))));
    }

    // WT.13 — query respects limit
    #[tokio::test]
    async fn wiki_tool_query_respects_limit() {
        let (_dir, tool) = setup();
        // Only 1 page in the test wiki — limit=1 should return it
        let result = tool
            .execute(serde_json::json!({
                "action": "query",
                "question": "attention",
                "limit": 1
            }))
            .await
            .unwrap();

        assert!(result.success);
        // Should contain page heading separator format
        assert!(result.output.contains("###"));
    }

    // WT.7 — metadata: name, description, schema includes query action
    #[test]
    fn wiki_tool_metadata() {
        let wiki = Arc::new(Mutex::new(
            WikiManager::new(tempfile::TempDir::new().unwrap().path().to_path_buf()).unwrap(),
        ));
        let tool = WikiSearchTool::new(wiki);

        assert_eq!(tool.name(), "wiki");
        assert!(tool.description().contains("query"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("action")));

        let action_enum = schema["properties"]["action"]["enum"].as_array().unwrap();
        assert!(action_enum.contains(&serde_json::json!("query")));
        assert!(schema["properties"]["question"].is_object());
    }
}
