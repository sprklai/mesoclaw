use clap::Subcommand;

use crate::client::ZeniiClient;

#[derive(Subcommand, Debug)]
pub enum WikiAction {
    /// List all wiki pages
    List,
    /// Search wiki pages
    Search {
        /// Search query
        query: String,
    },
    /// Show a wiki page by slug
    Show {
        /// Page slug (filename without .md)
        slug: String,
    },
    /// Sync wiki page TLDRs to memory
    Sync,
}

pub async fn list(client: &ZeniiClient) -> Result<(), String> {
    let pages: Vec<serde_json::Value> = client.get("/wiki").await?;
    if pages.is_empty() {
        println!("No wiki pages found.");
    } else {
        for page in &pages {
            let slug = page.get("slug").and_then(|v| v.as_str()).unwrap_or("?");
            let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("(no title)");
            let page_type = page.get("page_type").and_then(|v| v.as_str()).unwrap_or("");
            println!("[{page_type}] {slug} — {title}");
        }
        println!("\n{} page(s)", pages.len());
    }
    Ok(())
}

pub async fn search(client: &ZeniiClient, query: &str) -> Result<(), String> {
    let path = format!("/wiki/search?q={}", urlencoded(query));
    let pages: Vec<serde_json::Value> = client.get(&path).await?;
    if pages.is_empty() {
        println!("No results for '{query}'.");
    } else {
        for page in &pages {
            let slug = page.get("slug").and_then(|v| v.as_str()).unwrap_or("?");
            let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("(no title)");
            println!("{slug} — {title}");
        }
    }
    Ok(())
}

pub async fn show(client: &ZeniiClient, slug: &str) -> Result<(), String> {
    let page: serde_json::Value = client.get(&format!("/wiki/{slug}")).await?;
    let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("(no title)");
    let page_type = page.get("page_type").and_then(|v| v.as_str()).unwrap_or("");
    let updated = page.get("updated").and_then(|v| v.as_str()).unwrap_or("");
    let tldr = page.get("tldr").and_then(|v| v.as_str()).unwrap_or("");
    let body = page.get("body").and_then(|v| v.as_str()).unwrap_or("");

    println!("# {title}");
    println!("Type: {page_type} | Updated: {updated}");

    if let Some(tags) = page.get("tags").and_then(|v| v.as_array()) {
        let tag_strs: Vec<&str> = tags.iter().filter_map(|t| t.as_str()).collect();
        if !tag_strs.is_empty() {
            println!("Tags: {}", tag_strs.join(", "));
        }
    }

    if !tldr.is_empty() {
        println!("\n## TLDR\n{tldr}");
    }
    if !body.is_empty() {
        println!("\n{body}");
    }
    Ok(())
}

pub async fn sync(client: &ZeniiClient) -> Result<(), String> {
    let result: serde_json::Value = client.post("/wiki/sync", &serde_json::json!({})).await?;
    let count = result.get("synced").and_then(|v| v.as_u64()).unwrap_or(0);
    println!("Synced {count} wiki page(s) to memory.");
    Ok(())
}

fn urlencoded(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => result.push_str(&format!("%{byte:02X}")),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Minimal parser used only for unit-testing `WikiAction` parsing.
    #[derive(Parser)]
    #[command(name = "test")]
    struct TestCli {
        #[command(subcommand)]
        action: WikiAction,
    }

    #[test]
    fn wiki_list_parses() {
        let cli = TestCli::parse_from(["test", "list"]);
        assert!(matches!(cli.action, WikiAction::List));
    }

    #[test]
    fn wiki_search_parses_query() {
        let cli = TestCli::parse_from(["test", "search", "my query"]);
        match cli.action {
            WikiAction::Search { query } => assert_eq!(query, "my query"),
            other => panic!("expected WikiAction::Search, got {other:?}"),
        }
    }

    #[test]
    fn wiki_show_parses_slug() {
        let cli = TestCli::parse_from(["test", "show", "some-page"]);
        match cli.action {
            WikiAction::Show { slug } => assert_eq!(slug, "some-page"),
            other => panic!("expected WikiAction::Show, got {other:?}"),
        }
    }

    #[test]
    fn wiki_sync_parses() {
        let cli = TestCli::parse_from(["test", "sync"]);
        assert!(matches!(cli.action, WikiAction::Sync));
    }
}
