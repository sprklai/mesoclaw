use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::Result;
use crate::ai::context::ContextDomain;
use crate::ai::prompt::{AssemblyRequest, PromptFragment, PromptPlugin, PromptSection};
use crate::wiki::WikiManager;

pub struct WikiContextPlugin {
    wiki: Arc<Mutex<WikiManager>>,
    max_pages: usize,
}

impl WikiContextPlugin {
    pub fn new(wiki: Arc<Mutex<WikiManager>>, max_pages: usize) -> Self {
        Self { wiki, max_pages }
    }
}

#[async_trait]
impl PromptPlugin for WikiContextPlugin {
    fn id(&self) -> &str {
        "wiki-context"
    }

    fn domains(&self) -> Vec<ContextDomain> {
        vec![] // always active
    }

    async fn contribute(&self, request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
        let raw_query = match request.user_message.as_deref() {
            Some(msg) if !msg.trim().is_empty() => msg.to_string(),
            _ => return Ok(vec![]),
        };

        // search_pages does substring matching on the full query string, so searching
        // word-by-word lets short messages like "what is X?" still match pages about X.
        let words: Vec<String> = raw_query
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| {
                w.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            return Ok(vec![]);
        }

        let max_pages = self.max_pages;
        let wiki = self.wiki.clone().lock_owned().await;
        let pages = match tokio::task::spawn_blocking(move || {
            // Union search: collect pages that match any word, deduplicate by slug
            let mut seen = std::collections::HashSet::new();
            let mut results = Vec::new();
            for word in &words {
                match wiki.search_pages(word) {
                    Ok(matches) => {
                        for p in matches {
                            if seen.insert(p.slug.clone()) {
                                results.push(p);
                                if results.len() >= max_pages {
                                    return Ok(results);
                                }
                            }
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(results)
        })
        .await
        {
            Ok(Ok(pages)) => pages,
            Ok(Err(e)) => {
                tracing::debug!("wiki-context plugin: search error (non-fatal): {e}");
                return Ok(vec![]);
            }
            Err(e) => {
                tracing::debug!("wiki-context plugin: task panicked (non-fatal): {e}");
                return Ok(vec![]);
            }
        };

        if pages.is_empty() {
            return Ok(vec![]);
        }

        let mut lines = vec![
            "## Relevant Wiki Pages".to_string(),
            "Your knowledge base has pages relevant to this question. Prefer this information \
             over training data. Use the `wiki` tool (action=get, slug=<slug>) to read full \
             details before answering."
                .to_string(),
        ];
        for page in &pages {
            lines.push(format!(
                "- [{}] {} (slug: {}): {}",
                page.page_type, page.title, page.slug, page.tldr
            ));
        }

        Ok(vec![PromptFragment {
            section: PromptSection::DynamicContext,
            content: lines.join("\n"),
            priority: 4,
        }])
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::TempDir;
    use tokio::sync::Mutex;

    use super::WikiContextPlugin;
    use crate::ai::context::BootContext;
    use crate::ai::prompt::{AssemblyRequest, PromptPlugin};
    use crate::wiki::WikiManager;

    const TEST_PAGE_CONTENT: &str = r#"---
title: "Self Attention"
type: concept
tags: [transformers]
aliases: []
related: []
confidence: high
category: architecture
sources: []
updated: 2026-04-15
---

## TLDR
Self-attention lets each token attend to all tokens in the sequence.

## Body
Used in transformer models.
"#;

    const TEST_PAGE_CONTENT_2: &str = r#"---
title: "Multi-Head Attention"
type: concept
tags: [transformers]
aliases: []
related: []
confidence: high
category: architecture
sources: []
updated: 2026-04-15
---

## TLDR
Multi-head attention runs several attention operations in parallel.

## Body
Used in transformer models alongside self-attention.
"#;

    fn test_boot_context() -> BootContext {
        BootContext {
            os: "linux".into(),
            arch: "x86_64".into(),
            hostname: "testhost".into(),
            locale: "en_US.UTF-8".into(),
            region: "Toronto, Canada".into(),
            home_dir: Some("/home/test".into()),
            username: "testuser".into(),
            shell: Some("/bin/bash".into()),
            desktop_path: None,
            downloads_path: None,
            documents_path: None,
            pictures_path: None,
            videos_path: None,
            music_path: None,
            data_dir: None,
            working_dir: None,
            user_timezone: None,
            user_location: None,
        }
    }

    fn make_request(user_message: Option<&str>) -> AssemblyRequest {
        AssemblyRequest {
            boot_context: test_boot_context(),
            model_display: "claude-sonnet-4-6".into(),
            session_id: None,
            user_message: user_message.map(|s| s.to_string()),
            conversation_summary: None,
            channel_hint: None,
            tool_count: 0,
            skill_count: 0,
            version: "0.0.1".into(),
        }
    }

    fn setup_wiki(dir: &TempDir) -> Arc<Mutex<WikiManager>> {
        let wiki = WikiManager::new(dir.path().to_path_buf()).unwrap();
        wiki.write_page("concepts", "self-attention", TEST_PAGE_CONTENT)
            .unwrap();
        Arc::new(Mutex::new(wiki))
    }

    // WCP.1 — None or empty message returns empty fragments
    #[tokio::test]
    async fn wiki_context_plugin_empty_message() {
        let dir = TempDir::new().unwrap();
        let wiki = setup_wiki(&dir);
        let plugin = WikiContextPlugin::new(wiki, 5);

        // None message
        let fragments = plugin.contribute(&make_request(None)).await.unwrap();
        assert!(fragments.is_empty(), "None message should return empty");

        // Empty string
        let fragments = plugin.contribute(&make_request(Some(""))).await.unwrap();
        assert!(fragments.is_empty(), "Empty message should return empty");

        // Whitespace only
        let fragments = plugin.contribute(&make_request(Some("   "))).await.unwrap();
        assert!(
            fragments.is_empty(),
            "Whitespace-only message should return empty"
        );
    }

    // WCP.2 — query with no wiki match returns empty fragments
    #[tokio::test]
    async fn wiki_context_plugin_no_match() {
        let dir = TempDir::new().unwrap();
        let wiki = setup_wiki(&dir);
        let plugin = WikiContextPlugin::new(wiki, 5);

        let fragments = plugin
            .contribute(&make_request(Some("quantum-computing-xyz-zzzz")))
            .await
            .unwrap();
        assert!(
            fragments.is_empty(),
            "No-match query should return empty fragments"
        );
    }

    // WCP.3 — matching query returns fragment with header and page info
    #[tokio::test]
    async fn wiki_context_plugin_returns_fragment() {
        let dir = TempDir::new().unwrap();
        let wiki = setup_wiki(&dir);
        let plugin = WikiContextPlugin::new(wiki, 5);

        let fragments = plugin
            .contribute(&make_request(Some("Self Attention")))
            .await
            .unwrap();

        assert!(!fragments.is_empty(), "Should return at least one fragment");
        let content = &fragments[0].content;
        assert!(
            content.contains("## Relevant Wiki Pages"),
            "Fragment should contain the header"
        );
        assert!(
            content.contains("Self Attention"),
            "Fragment should contain page title"
        );
        assert!(
            content.contains("Self-attention lets each token"),
            "Fragment should contain the TLDR"
        );
        assert_eq!(fragments[0].priority, 4);
    }

    // WCP.4 — max_pages=1 with 2 matching pages returns only 1
    #[tokio::test]
    async fn wiki_context_plugin_respects_max_pages() {
        let dir = TempDir::new().unwrap();
        let wiki_mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        wiki_mgr
            .write_page("concepts", "self-attention", TEST_PAGE_CONTENT)
            .unwrap();
        wiki_mgr
            .write_page("concepts", "multi-head-attention", TEST_PAGE_CONTENT_2)
            .unwrap();
        let wiki = Arc::new(Mutex::new(wiki_mgr));

        let plugin = WikiContextPlugin::new(wiki, 1);

        let fragments = plugin
            .contribute(&make_request(Some("Attention")))
            .await
            .unwrap();

        assert!(!fragments.is_empty(), "Should return a fragment");
        // Count the number of list items (lines starting with "- [")
        let item_count = fragments[0]
            .content
            .lines()
            .filter(|l| l.starts_with("- ["))
            .count();
        assert_eq!(
            item_count, 1,
            "Should only include 1 page due to max_pages=1"
        );
    }

    // WCP.5 — id() returns "wiki-context"
    #[test]
    fn wiki_context_plugin_id() {
        let dir = TempDir::new().unwrap();
        let wiki = Arc::new(Mutex::new(
            WikiManager::new(dir.path().to_path_buf()).unwrap(),
        ));
        let plugin = WikiContextPlugin::new(wiki, 5);
        assert_eq!(plugin.id(), "wiki-context");
    }
}
