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
    /// Ingest a local file into the wiki (LLM-driven multi-page generation)
    Ingest {
        /// Path to the file to ingest (markdown, text, HTML, etc.)
        file: String,
        /// Optional model override (e.g. gpt-4o, claude-3-5-sonnet)
        #[arg(long)]
        model: Option<String>,
    },
    /// Ask a question answered from wiki knowledge
    Query {
        /// The question to answer
        question: String,
        /// Save the answer as a query page in the wiki
        #[arg(long)]
        save: bool,
        /// Optional model override
        #[arg(long)]
        model: Option<String>,
    },
    /// Lint all wiki pages for structural issues
    Lint,
    /// List all ingested source documents
    Sources,
    /// Delete a source document and cascade-cleanup its derived pages
    DeleteSource {
        /// Filename of the source to delete (e.g. paper.md)
        filename: String,
        /// Optional model override for rebuilding shared pages
        #[arg(long)]
        model: Option<String>,
    },
    /// Regenerate all wiki pages from saved sources (use after editing INGEST_PROMPT.md or changing model)
    Regenerate {
        /// Optional model override
        #[arg(long)]
        model: Option<String>,
    },
    /// Show or change the ingest prompt
    Prompt {
        #[command(subcommand)]
        action: PromptAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum PromptAction {
    /// Print the current ingest prompt
    Show,
    /// Set the ingest prompt to the given text
    Set {
        /// New prompt content
        content: String,
    },
    /// Open the prompt file in $VISUAL or $EDITOR
    Edit,
}

pub async fn list(client: &ZeniiClient) -> Result<(), String> {
    let data: serde_json::Value = client.get("/wiki?limit=200&offset=0").await?;
    let empty = vec![];
    let pages = data["pages"].as_array().unwrap_or(&empty);
    if pages.is_empty() {
        println!("No wiki pages found.");
    } else {
        for page in pages {
            let slug = page.get("slug").and_then(|v| v.as_str()).unwrap_or("?");
            let title = page
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(no title)");
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
            let title = page
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(no title)");
            println!("{slug} — {title}");
        }
    }
    Ok(())
}

pub async fn show(client: &ZeniiClient, slug: &str) -> Result<(), String> {
    let page: serde_json::Value = client.get(&format!("/wiki/{slug}")).await?;
    let title = page
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("(no title)");
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

pub async fn ingest(client: &ZeniiClient, file: &str, model: Option<&str>) -> Result<(), String> {
    let path = std::path::Path::new(file);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("invalid file path: {file}"))?
        .to_string();
    // Read as raw bytes — works for both text and binary formats.
    // The daemon's DocumentConverter handles conversion based on file extension.
    let bytes = std::fs::read(path).map_err(|e| format!("failed to read '{file}': {e}"))?;

    println!("Ingesting '{filename}'… (this may take 30–120s with LLM)");

    let result: serde_json::Value = client
        .upload_file("/wiki/upload", bytes, filename, model)
        .await?;

    let primary_slug = result
        .get("primary_slug")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let message = result
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("done");
    println!("{message}");

    if let Some(pages) = result.get("pages").and_then(|v| v.as_array()) {
        for page in pages {
            let slug = page.get("slug").and_then(|v| v.as_str()).unwrap_or("?");
            let title = page
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(no title)");
            let page_type = page.get("page_type").and_then(|v| v.as_str()).unwrap_or("");
            println!("  [{page_type}] {slug} — {title}");
        }
    }
    println!("Primary slug: {primary_slug}");
    Ok(())
}

pub async fn query(
    client: &ZeniiClient,
    question: &str,
    save: bool,
    model: Option<&str>,
) -> Result<(), String> {
    let mut body = serde_json::json!({ "question": question, "save": save });
    if let Some(m) = model {
        body["model"] = serde_json::Value::String(m.to_string());
    }

    let result: serde_json::Value = client.post("/wiki/query", &body).await?;

    let answer = result.get("answer").and_then(|v| v.as_str()).unwrap_or("");
    println!("{answer}");

    if let Some(citations) = result.get("citations").and_then(|v| v.as_array()) {
        let cite_strs: Vec<&str> = citations.iter().filter_map(|v| v.as_str()).collect();
        if !cite_strs.is_empty() {
            println!("\nCitations:");
            for c in cite_strs {
                println!("  [[{c}]]");
            }
        }
    }

    if let Some(saved) = result.get("saved_page").filter(|v| !v.is_null()) {
        let slug = saved.get("slug").and_then(|v| v.as_str()).unwrap_or("?");
        println!("\nSaved as wiki page: {slug}");
    }
    Ok(())
}

pub async fn lint(client: &ZeniiClient) -> Result<(), String> {
    println!("Running wiki lint…");
    let result: serde_json::Value = client.post("/wiki/lint", &serde_json::json!({})).await?;

    let summary = result.get("summary").and_then(|v| v.as_str()).unwrap_or("");
    let issues = result
        .get("issues")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    println!("{summary}\n");
    for issue in &issues {
        let kind = issue.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
        let page = issue
            .get("page_slug")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let detail = issue.get("detail").and_then(|v| v.as_str()).unwrap_or("");
        println!("[{kind}] {page}: {detail}");
    }
    Ok(())
}

pub async fn sources(client: &ZeniiClient) -> Result<(), String> {
    let records: Vec<serde_json::Value> = client.get("/wiki/sources").await?;
    if records.is_empty() {
        println!("No sources ingested yet.");
    } else {
        for rec in &records {
            let filename = rec.get("filename").and_then(|v| v.as_str()).unwrap_or("?");
            let hash = rec.get("hash").and_then(|v| v.as_str()).unwrap_or("");
            let active = rec.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
            let run_id = rec
                .get("last_run_id")
                .and_then(|v| v.as_str())
                .unwrap_or("—");
            let status = if active { "active" } else { "inactive" };
            println!(
                "{filename}  [{status}]  run={run_id}  sha256={}",
                &hash[..12.min(hash.len())]
            );
        }
        println!("\n{} source(s)", records.len());
    }
    Ok(())
}

pub async fn delete_source(
    client: &ZeniiClient,
    filename: &str,
    model: Option<&str>,
) -> Result<(), String> {
    let mut path = format!("/wiki/sources/{}", urlencoded(filename));
    if let Some(m) = model {
        path = format!("{path}?model={}", urlencoded(m));
    }
    println!("Deleting source '{filename}' and its exclusive pages…");
    let result: serde_json::Value = client.delete_json(&path).await?;
    let deleted: Vec<&str> = result
        .get("deleted_pages")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let rebuilt: Vec<&str> = result
        .get("rebuilt_pages")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    if !deleted.is_empty() {
        println!("Deleted pages: {}", deleted.join(", "));
    }
    if !rebuilt.is_empty() {
        println!("Rebuilt pages: {}", rebuilt.join(", "));
    }
    if deleted.is_empty() && rebuilt.is_empty() {
        println!("No pages were affected.");
    }
    Ok(())
}

pub async fn regenerate(client: &ZeniiClient, model: Option<&str>) -> Result<(), String> {
    let mut body = serde_json::json!({});
    if let Some(m) = model {
        body["model"] = serde_json::Value::String(m.to_string());
    }
    println!("Regenerating wiki from all sources… (this may take several minutes)");
    let result: serde_json::Value = client.post("/wiki/regenerate", &body).await?;
    let message = result
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("done");
    println!("{message}");
    Ok(())
}

pub async fn prompt(client: &ZeniiClient, action: &PromptAction) -> Result<(), String> {
    match action {
        PromptAction::Show => {
            let content = client.wiki_prompt_get().await?;
            println!("{content}");
        }
        PromptAction::Set { content } => {
            client.wiki_prompt_set(content).await?;
            println!("Prompt updated.");
        }
        PromptAction::Edit => {
            let current = client.wiki_prompt_get().await?;
            let tmp = tempfile::Builder::new()
                .suffix(".md")
                .tempfile()
                .map_err(|e| e.to_string())?;
            std::fs::write(tmp.path(), &current).map_err(|e| e.to_string())?;
            let editor_str = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "nano".to_string());
            let mut parts = editor_str.split_whitespace();
            let program = parts
                .next()
                .ok_or_else(|| "EDITOR variable is empty".to_string())?;
            let editor_args: Vec<&str> = parts.collect();
            let status = tokio::process::Command::new(program)
                .args(&editor_args)
                .arg(tmp.path())
                .status()
                .await
                .map_err(|e| format!("Failed to launch editor '{program}': {e}"))?;
            if !status.success() {
                return Err(format!("Editor exited with status {status}"));
            }
            let new_content = std::fs::read_to_string(tmp.path()).map_err(|e| e.to_string())?;
            if new_content == current {
                println!("No changes.");
                return Ok(());
            }
            client.wiki_prompt_set(&new_content).await?;
            println!("Prompt updated.");
        }
    }
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

    #[test]
    fn wiki_ingest_parses_file_and_model() {
        let cli = TestCli::parse_from(["test", "ingest", "my-doc.md", "--model", "gpt-4o"]);
        match cli.action {
            WikiAction::Ingest { file, model } => {
                assert_eq!(file, "my-doc.md");
                assert_eq!(model.as_deref(), Some("gpt-4o"));
            }
            other => panic!("expected WikiAction::Ingest, got {other:?}"),
        }
    }

    #[test]
    fn wiki_ingest_parses_file_only() {
        let cli = TestCli::parse_from(["test", "ingest", "notes.txt"]);
        match cli.action {
            WikiAction::Ingest { file, model } => {
                assert_eq!(file, "notes.txt");
                assert!(model.is_none());
            }
            other => panic!("expected WikiAction::Ingest, got {other:?}"),
        }
    }

    #[test]
    fn wiki_query_parses_question_and_flags() {
        let cli = TestCli::parse_from([
            "test",
            "query",
            "What is attention?",
            "--save",
            "--model",
            "claude-3-5-sonnet",
        ]);
        match cli.action {
            WikiAction::Query {
                question,
                save,
                model,
            } => {
                assert_eq!(question, "What is attention?");
                assert!(save);
                assert_eq!(model.as_deref(), Some("claude-3-5-sonnet"));
            }
            other => panic!("expected WikiAction::Query, got {other:?}"),
        }
    }

    #[test]
    fn wiki_query_parses_question_only() {
        let cli = TestCli::parse_from(["test", "query", "What is RAG?"]);
        match cli.action {
            WikiAction::Query {
                question,
                save,
                model,
            } => {
                assert_eq!(question, "What is RAG?");
                assert!(!save);
                assert!(model.is_none());
            }
            other => panic!("expected WikiAction::Query, got {other:?}"),
        }
    }

    #[test]
    fn wiki_lint_parses() {
        let cli = TestCli::parse_from(["test", "lint"]);
        assert!(matches!(cli.action, WikiAction::Lint));
    }
}
