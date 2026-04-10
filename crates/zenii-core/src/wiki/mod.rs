use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ZeniiError;

// Embedded SCHEMA.md template — seeded into new wiki dirs on first boot.
const SCHEMA_TEMPLATE: &str = include_str!("../../../../wiki/SCHEMA.md");
// Embedded INGEST_PROMPT.md template — seeded once, then user-editable.
const INGEST_PROMPT_TEMPLATE: &str = include_str!("../../../../wiki/INGEST_PROMPT.md");

// ── Data types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub slug: String,
    pub title: String,
    pub page_type: String, // concept|entity|topic|comparison|query
    pub tags: Vec<String>,
    pub sources: Vec<String>,
    pub updated: String,        // YYYY-MM-DD
    pub tldr: String,           // content after ## TLDR heading
    pub body: String,           // full raw markdown
    pub wikilinks: Vec<String>, // extracted [[slug]] targets
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiGraph {
    pub nodes: Vec<WikiNode>,
    pub edges: Vec<WikiEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiNode {
    pub id: String,
    pub label: String,
    pub page_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintIssue {
    pub kind: String, // "broken_wikilink" | "orphan_page" | "missing_index_entry" | "missing_updated"
    pub page_slug: String,
    pub detail: String,
    pub fix: String,
}

// ── Manifest types ───────────────────────────────────────────────────────────

/// One record per raw source file in wiki/sources/.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub filename: String,
    pub hash: String, // SHA-256 hex of file content
    pub active: bool,
    pub last_run_id: Option<String>,
}

/// One record per generated page in wiki/pages/.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRecord {
    pub slug: String,
    pub page_type: String,
    pub path: String,              // relative to wiki_dir: "pages/concepts/foo.md"
    pub sources: Vec<String>,      // contributing source filenames
    pub last_run_id: String,
    pub managed_by: String,        // "source_ingest" | "user_query"
}

/// One record per compiler run appended to wiki/.meta/runs.jsonl.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub run_id: String,
    pub timestamp: String,
    pub model: Option<String>,
    pub prompt_hash: String,  // SHA-256 of INGEST_PROMPT.md content
    pub schema_hash: String,  // SHA-256 of SCHEMA.md content
    pub sources: Vec<String>, // source filenames included in this run
    pub status: String,       // "success" | "failed"
    pub pages_written: Vec<String>,
}

/// Result of a delete-source operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSourceResult {
    pub filename: String,
    pub deleted_pages: Vec<String>,
    pub rebuilt_pages: Vec<String>,
}

// ── Subdirectories created on init ──────────────────────────────────────────

const PAGE_SUBDIRS: &[&str] = &["concepts", "entities", "topics", "comparisons", "queries"];

// ── WikiManager ──────────────────────────────────────────────────────────────

pub struct WikiManager {
    wiki_dir: PathBuf,
}

impl WikiManager {
    pub fn new(wiki_dir: PathBuf) -> Result<Self, ZeniiError> {
        // Page subdirectories
        let pages_dir = wiki_dir.join("pages");
        for subdir in PAGE_SUBDIRS {
            std::fs::create_dir_all(pages_dir.join(subdir))?;
        }
        // sources/ directory
        std::fs::create_dir_all(wiki_dir.join("sources"))?;
        // .meta/ directory for manifest files
        std::fs::create_dir_all(wiki_dir.join(".meta"))?;
        // SCHEMA.md — seed from embedded template, never overwrite existing
        let schema_path = wiki_dir.join("SCHEMA.md");
        if !schema_path.exists() {
            std::fs::write(&schema_path, SCHEMA_TEMPLATE)?;
        }
        // INGEST_PROMPT.md — seed from embedded template, never overwrite (user-editable)
        let prompt_path = wiki_dir.join("INGEST_PROMPT.md");
        if !prompt_path.exists() {
            std::fs::write(&prompt_path, INGEST_PROMPT_TEMPLATE)?;
        }
        // index.md stub on first boot
        let index_path = wiki_dir.join("index.md");
        if !index_path.exists() {
            std::fs::write(
                &index_path,
                "# Wiki Index\n<!-- LLM maintains this file. Do not edit manually. -->\n\n_No pages yet. Ingest your first source to get started._\n",
            )?;
        }
        // log.md stub on first boot
        let log_path = wiki_dir.join("log.md");
        if !log_path.exists() {
            std::fs::write(
                &log_path,
                "# Wiki Log\n<!-- Append-only. LLM appends entries after each operation. -->\n",
            )?;
        }
        Ok(Self { wiki_dir })
    }

    /// Return the root wiki directory path.
    pub fn wiki_dir(&self) -> &Path {
        &self.wiki_dir
    }

    pub fn list_pages(&self) -> Result<Vec<WikiPage>, ZeniiError> {
        let pages_dir = self.wiki_dir.join("pages");
        let mut pages = Vec::new();
        walk_pages_dir(&pages_dir, &mut pages)?;
        Ok(pages)
    }

    pub fn get_page(&self, slug: &str) -> Result<Option<WikiPage>, ZeniiError> {
        let pages_dir = self.wiki_dir.join("pages");
        find_page_in_dir(&pages_dir, slug)
    }

    pub fn search_pages(&self, query: &str) -> Result<Vec<WikiPage>, ZeniiError> {
        let all = self.list_pages()?;
        if query.is_empty() {
            return Ok(all);
        }
        let q = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|p| p.title.to_lowercase().contains(&q) || p.body.to_lowercase().contains(&q))
            .collect())
    }

    pub async fn sync_to_memory(
        &self,
        memory: &dyn crate::memory::traits::Memory,
    ) -> Result<usize, ZeniiError> {
        let pages = self.list_pages()?;
        let mut count = 0;
        for page in pages {
            let content = if !page.tldr.trim().is_empty() {
                page.tldr.clone()
            } else if !page.title.trim().is_empty() {
                page.title.clone()
            } else {
                continue;
            };
            let key = format!("wiki:{}", page.slug);
            memory
                .store(
                    &key,
                    &content,
                    crate::memory::traits::MemoryCategory::Custom("wiki".into()),
                )
                .await?;
            count += 1;
        }
        Ok(count)
    }

    pub fn graph(&self) -> Result<WikiGraph, ZeniiError> {
        let pages = self.list_pages()?;
        let nodes = pages
            .iter()
            .map(|p| WikiNode {
                id: p.slug.clone(),
                label: p.title.clone(),
                page_type: p.page_type.clone(),
            })
            .collect();
        let edges = pages
            .iter()
            .flat_map(|p| {
                p.wikilinks.iter().map(move |link| WikiEdge {
                    from: p.slug.clone(),
                    to: link.clone(),
                })
            })
            .collect();
        Ok(WikiGraph { nodes, edges })
    }

    /// Write content as a new wiki page (used by ingest handler).
    /// Converts filename to a slug, writes to pages/topics/{slug}.md.
    pub fn ingest(&self, filename: &str, content: &str) -> Result<WikiPage, ZeniiError> {
        let slug = filename
            .trim_end_matches(".md")
            .to_lowercase()
            .replace(' ', "-");
        let page_path = self
            .wiki_dir
            .join("pages")
            .join("topics")
            .join(format!("{slug}.md"));
        let name_hint = filename.trim_end_matches(".md").trim().to_string();
        std::fs::write(&page_path, content)?;
        parse_page(slug, &page_path, Some(&name_hint))
    }

    /// Write a wiki page to the appropriate subdirectory for the given type.
    /// `page_type` must be one of: concepts, entities, topics, comparisons, queries.
    pub fn write_page(
        &self,
        page_type: &str,
        slug: &str,
        content: &str,
    ) -> Result<WikiPage, ZeniiError> {
        if !PAGE_SUBDIRS.contains(&page_type) {
            return Err(ZeniiError::Validation(format!(
                "invalid page type '{page_type}'; must be one of: {}",
                PAGE_SUBDIRS.join(", ")
            )));
        }
        let page_path = self
            .wiki_dir
            .join("pages")
            .join(page_type)
            .join(format!("{slug}.md"));
        std::fs::write(&page_path, content)?;
        parse_page(slug.to_string(), &page_path, None)
    }

    /// Save raw source content to wiki/sources/{filename}.
    pub fn save_source(&self, filename: &str, content: &str) -> Result<(), ZeniiError> {
        let sources_dir = self.wiki_dir.join("sources");
        std::fs::create_dir_all(&sources_dir)?;
        std::fs::write(sources_dir.join(filename), content)?;
        Ok(())
    }

    /// Rewrite wiki/index.md from the current set of pages, grouped by type per SCHEMA.
    pub fn update_index(&self) -> Result<(), ZeniiError> {
        let pages = self.list_pages()?;
        // SCHEMA-specified section order (singular page_type → heading label)
        const SECTION_ORDER: &[(&str, &str)] = &[
            ("concept", "Concepts"),
            ("entity", "Entities"),
            ("topic", "Topics"),
            ("comparison", "Comparisons"),
            ("query", "Queries"),
        ];
        let mut lines = vec![
            "# Wiki Index".to_string(),
            "<!-- LLM maintains this file. Do not edit manually. -->".to_string(),
            String::new(),
        ];
        if pages.is_empty() {
            lines.push("_No pages yet. Ingest your first source to get started._".to_string());
        } else {
            for (type_key, heading) in SECTION_ORDER {
                let section: Vec<&WikiPage> =
                    pages.iter().filter(|p| p.page_type == *type_key).collect();
                if section.is_empty() {
                    continue;
                }
                lines.push(format!("## {heading}"));
                for page in section {
                    let tldr = if page.tldr.is_empty() {
                        "—".to_string()
                    } else {
                        page.tldr.lines().next().unwrap_or("—").to_string()
                    };
                    lines.push(format!("- [[{}]] — {}", page.slug, tldr));
                }
                lines.push(String::new());
            }
        }
        std::fs::write(self.wiki_dir.join("index.md"), lines.join("\n") + "\n")?;
        Ok(())
    }

    /// Read the current wiki/index.md contents (for use as LLM context in ingest/query).
    pub fn read_index(&self) -> Result<String, ZeniiError> {
        let path = self.wiki_dir.join("index.md");
        if path.exists() {
            Ok(std::fs::read_to_string(&path)?)
        } else {
            Ok(String::new())
        }
    }

    /// Run deterministic structural lint over all wiki pages.
    /// Returns a list of issues found. Does NOT call the LLM.
    pub fn lint(&self) -> Result<Vec<LintIssue>, ZeniiError> {
        let pages = self.list_pages()?;
        let mut issues: Vec<LintIssue> = Vec::new();

        // Build a set of all known slugs for O(1) lookup
        let known_slugs: std::collections::HashSet<&str> =
            pages.iter().map(|p| p.slug.as_str()).collect();

        // Build inbound-link count map: slug → number of other pages linking here
        let mut inbound: std::collections::HashMap<&str, usize> =
            pages.iter().map(|p| (p.slug.as_str(), 0usize)).collect();
        for page in &pages {
            for link in &page.wikilinks {
                if let Some(count) = inbound.get_mut(link.as_str()) {
                    *count += 1;
                }
            }
        }

        // Extract slugs listed in index.md
        let index_content = self.read_index()?;
        let mut indexed_slugs: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut rest = index_content.as_str();
        while let Some(open) = rest.find("[[") {
            rest = &rest[open + 2..];
            if let Some(close) = rest.find("]]") {
                indexed_slugs.insert(rest[..close].to_string());
                rest = &rest[close + 2..];
            } else {
                break;
            }
        }

        for page in &pages {
            // Lint 1: broken wikilinks
            for link in &page.wikilinks {
                if !known_slugs.contains(link.as_str()) {
                    issues.push(LintIssue {
                        kind: "broken_wikilink".to_string(),
                        page_slug: page.slug.clone(),
                        detail: format!("[[{link}]] has no matching page file"),
                        fix: format!(
                            "Remove or correct the [[{link}]] reference in '{}', or create a page with slug '{link}'.",
                            page.slug
                        ),
                    });
                }
            }

            // Lint 2: orphan pages (skip for single-page wikis)
            if pages.len() > 1 {
                let count = inbound.get(page.slug.as_str()).copied().unwrap_or(0);
                if count == 0 {
                    issues.push(LintIssue {
                        kind: "orphan_page".to_string(),
                        page_slug: page.slug.clone(),
                        detail: format!(
                            "'{}' has no incoming wikilinks from any other page",
                            page.slug
                        ),
                        fix: format!(
                            "Add a [[{}]] wikilink from at least one other page, or delete this page if it is a stub.",
                            page.slug
                        ),
                    });
                }
            }

            // Lint 3: missing index entry
            if !indexed_slugs.contains(&page.slug) {
                issues.push(LintIssue {
                    kind: "missing_index_entry".to_string(),
                    page_slug: page.slug.clone(),
                    detail: format!("'{}' is not listed in index.md", page.slug),
                    fix: format!(
                        "Call POST /wiki/sync to regenerate the index, or add '- [[{}]]' manually to index.md.",
                        page.slug
                    ),
                });
            }

            // Lint 4: missing updated field
            if page.updated.trim().is_empty() {
                issues.push(LintIssue {
                    kind: "missing_updated".to_string(),
                    page_slug: page.slug.clone(),
                    detail: format!(
                        "'{}' has an empty or missing 'updated' frontmatter field",
                        page.slug
                    ),
                    fix: format!(
                        "Add 'updated: YYYY-MM-DD' to the YAML frontmatter of '{}'.",
                        page.slug
                    ),
                });
            }
        }

        Ok(issues)
    }

    // ── Manifest I/O ─────────────────────────────────────────────────────────

    /// Read the manifest from `.meta/sources.json` and `.meta/pages.json`.
    /// Returns empty vecs if either file is absent (bootstrap mode).
    pub fn read_manifest(&self) -> Result<(Vec<SourceRecord>, Vec<PageRecord>), ZeniiError> {
        let sources_path = self.wiki_dir.join(".meta").join("sources.json");
        let pages_path = self.wiki_dir.join(".meta").join("pages.json");
        let sources: Vec<SourceRecord> = if sources_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&sources_path)?)?
        } else {
            Vec::new()
        };
        let pages: Vec<PageRecord> = if pages_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&pages_path)?)?
        } else {
            Vec::new()
        };
        Ok((sources, pages))
    }

    /// Persist the manifest to `.meta/sources.json` and `.meta/pages.json`.
    pub fn write_manifest(
        &self,
        sources: &[SourceRecord],
        pages: &[PageRecord],
    ) -> Result<(), ZeniiError> {
        let meta_dir = self.wiki_dir.join(".meta");
        std::fs::create_dir_all(&meta_dir)?;
        std::fs::write(meta_dir.join("sources.json"), serde_json::to_string_pretty(sources)?)?;
        std::fs::write(meta_dir.join("pages.json"), serde_json::to_string_pretty(pages)?)?;
        Ok(())
    }

    /// Append one run record to the append-only `.meta/runs.jsonl`.
    pub fn append_run(&self, run: &RunRecord) -> Result<(), ZeniiError> {
        let path = self.wiki_dir.join(".meta").join("runs.jsonl");
        let line = format!("{}\n", serde_json::to_string(run)?);
        let existing = if path.exists() { std::fs::read_to_string(&path)? } else { String::new() };
        std::fs::write(&path, format!("{existing}{line}"))?;
        Ok(())
    }

    // ── Prompt ───────────────────────────────────────────────────────────────

    /// Read the current `INGEST_PROMPT.md` (user-editable generation instructions).
    pub fn read_ingest_prompt(&self) -> Result<String, ZeniiError> {
        let path = self.wiki_dir.join("INGEST_PROMPT.md");
        if path.exists() {
            Ok(std::fs::read_to_string(&path)?)
        } else {
            Ok(INGEST_PROMPT_TEMPLATE.to_string())
        }
    }

    /// Write new content to `wiki/INGEST_PROMPT.md`.
    /// Validates: non-empty, max 4000 chars.
    pub fn set_prompt(&self, content: &str) -> Result<(), ZeniiError> {
        if content.is_empty() {
            return Err(ZeniiError::Validation("prompt content cannot be empty".into()));
        }
        if content.chars().count() > 4000 {
            return Err(ZeniiError::Validation(
                "prompt content exceeds 4000 character limit".into(),
            ));
        }
        let path = self.wiki_dir.join("INGEST_PROMPT.md");
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Delete all `.md` files under `wiki/pages/` and reset `index.md` to empty stub.
    /// Does NOT touch `wiki/sources/`, `SCHEMA.md`, or `INGEST_PROMPT.md`.
    /// Returns the count of deleted page files.
    pub fn delete_all_pages(&self) -> Result<usize, ZeniiError> {
        let pages_dir = self.wiki_dir.join("pages");
        let count = delete_md_files_in_dir(&pages_dir)?;
        // Reset index.md to empty stub
        let index_path = self.wiki_dir.join("index.md");
        std::fs::write(
            &index_path,
            "# Wiki Index\n<!-- LLM maintains this file. Do not edit manually. -->\n\n_No pages yet. Ingest your first source to get started._\n",
        )?;
        // Clear pages from manifest
        let (sources, _pages) = self.read_manifest()?;
        self.write_manifest(&sources, &[])?;
        Ok(count)
    }

    // ── Sources ──────────────────────────────────────────────────────────────

    /// List all source files in `wiki/sources/`.
    /// Falls back to a filesystem scan if no manifest exists.
    pub fn list_sources(&self) -> Result<Vec<SourceRecord>, ZeniiError> {
        let (manifest_sources, _) = self.read_manifest()?;
        if !manifest_sources.is_empty() {
            return Ok(manifest_sources);
        }
        // Bootstrap: scan filesystem
        let sources_dir = self.wiki_dir.join("sources");
        if !sources_dir.exists() {
            return Ok(Vec::new());
        }
        let mut records = Vec::new();
        for entry in std::fs::read_dir(&sources_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let hash = Self::hash_content(&content);
                records.push(SourceRecord { filename, hash, active: true, last_run_id: None });
            }
        }
        Ok(records)
    }

    /// Read the raw content of a source file.
    pub fn read_source(&self, filename: &str) -> Result<String, ZeniiError> {
        let path = self.wiki_dir.join("sources").join(filename);
        if !path.exists() {
            return Err(ZeniiError::Validation(format!("source '{filename}' not found")));
        }
        Ok(std::fs::read_to_string(&path)?)
    }

    /// Delete a raw source file from `wiki/sources/`.
    pub fn delete_source_file(&self, filename: &str) -> Result<(), ZeniiError> {
        let path = self.wiki_dir.join("sources").join(filename);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Compute SHA-256 hex of content.
    pub fn hash_content(content: &str) -> String {
        let result = Sha256::digest(content.as_bytes());
        let bytes: &[u8] = result.as_ref();
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Generate a unique run ID based on current UTC timestamp.
    pub fn new_run_id() -> String {
        let now = chrono::Utc::now();
        let nanos = now.timestamp_nanos_opt().unwrap_or(now.timestamp_millis());
        format!("run-{}-{}", now.format("%Y%m%d"), &format!("{nanos:016x}")[10..])
    }

    // ── Staged build ─────────────────────────────────────────────────────────

    /// Create the staged build workspace at `wiki/.rebuild/`.
    /// Mirrors the `pages/` subdirectory structure.
    pub fn begin_staged_build(&self) -> Result<PathBuf, ZeniiError> {
        let rebuild_dir = self.wiki_dir.join(".rebuild");
        // Remove any leftover from a previous failed build
        if rebuild_dir.exists() {
            std::fs::remove_dir_all(&rebuild_dir)?;
        }
        for subdir in PAGE_SUBDIRS {
            std::fs::create_dir_all(rebuild_dir.join(subdir))?;
        }
        Ok(rebuild_dir)
    }

    /// Write a page into the staged workspace (does not touch live pages/).
    pub fn write_staged_page(
        &self,
        rebuild_dir: &Path,
        page_type: &str,
        slug: &str,
        content: &str,
    ) -> Result<(), ZeniiError> {
        if !PAGE_SUBDIRS.contains(&page_type) {
            return Err(ZeniiError::Validation(format!(
                "invalid page type '{page_type}'"
            )));
        }
        let path = rebuild_dir.join(page_type).join(format!("{slug}.md"));
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Move pages from the staged workspace into `wiki/pages/`, overwriting any
    /// existing files with the same slug/type.
    /// Returns a list of `(page_type, slug)` pairs that were committed.
    /// On success, removes the `.rebuild/` workspace.
    pub fn commit_staged_build(
        &self,
        rebuild_dir: &Path,
    ) -> Result<Vec<(String, String)>, ZeniiError> {
        let mut committed = Vec::new();
        for subdir in PAGE_SUBDIRS {
            let src_dir = rebuild_dir.join(subdir);
            if !src_dir.exists() {
                continue;
            }
            let dst_dir = self.wiki_dir.join("pages").join(subdir);
            std::fs::create_dir_all(&dst_dir)?;
            for entry in std::fs::read_dir(&src_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    let slug = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                    std::fs::copy(&path, dst_dir.join(path.file_name().unwrap()))?;
                    committed.push((subdir.to_string(), slug));
                }
            }
        }
        // Clean up staging workspace
        if rebuild_dir.exists() {
            std::fs::remove_dir_all(rebuild_dir)?;
        }
        Ok(committed)
    }

    /// Remove the staging workspace without touching live pages (called on failure).
    pub fn abort_staged_build(&self, rebuild_dir: &Path) {
        if rebuild_dir.exists() {
            let _ = std::fs::remove_dir_all(rebuild_dir);
        }
    }

    /// Delete the on-disk files for the given page records.
    /// Returns the number of files deleted.
    pub fn delete_page_files(&self, pages: &[PageRecord]) -> Result<usize, ZeniiError> {
        let mut count = 0;
        for record in pages {
            let path = self.wiki_dir.join(&record.path);
            if path.exists() {
                std::fs::remove_file(&path)?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Remove a specific source filename from the `sources:` frontmatter field of a page file.
    /// If the sources array becomes empty after removal, leaves it as empty.
    pub fn remove_source_from_page(
        &self,
        record: &PageRecord,
        filename: &str,
    ) -> Result<(), ZeniiError> {
        let path = self.wiki_dir.join(&record.path);
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        let updated = remove_source_from_frontmatter(&content, filename)?;
        std::fs::write(&path, updated)?;
        Ok(())
    }

    /// Append a log entry to wiki/log.md.
    pub fn append_log(&self, entry: &str) -> Result<(), ZeniiError> {
        let log_path = self.wiki_dir.join("log.md");
        let existing = if log_path.exists() {
            std::fs::read_to_string(&log_path)?
        } else {
            "# Wiki Log\n<!-- Append-only. LLM appends entries after each operation. -->\n"
                .to_string()
        };
        let updated = format!("{existing}\n{entry}\n");
        std::fs::write(&log_path, updated)?;
        Ok(())
    }
}

// ── Helper functions ─────────────────────────────────────────────────────────

/// Remove a specific source filename from the `sources:` YAML frontmatter array.
/// Returns the full file content with the updated frontmatter.
fn remove_source_from_frontmatter(content: &str, filename: &str) -> Result<String, ZeniiError> {
    let Some(rest) = content.strip_prefix("---") else {
        return Ok(content.to_string());
    };
    let Some(end_idx) = rest.find("\n---") else {
        return Ok(content.to_string());
    };
    let fm_str = &rest[..end_idx];
    let body = &rest[end_idx + 4..];

    let mut fm: serde_yaml::Value =
        serde_yaml::from_str(fm_str).unwrap_or(serde_yaml::Value::Mapping(Default::default()));

    if let serde_yaml::Value::Mapping(map) = &mut fm {
        let key = serde_yaml::Value::String("sources".to_string());
        if let Some(val) = map.get_mut(&key) {
            if let serde_yaml::Value::Sequence(seq) = val {
                seq.retain(|v| v.as_str() != Some(filename));
            }
        }
    }

    let new_fm = serde_yaml::to_string(&fm).map_err(|e| {
        ZeniiError::Validation(format!("failed to serialize frontmatter: {e}"))
    })?;
    // serde_yaml serializes with a leading "---\n" prefix — strip it to avoid double delimiter
    let new_fm = new_fm.strip_prefix("---\n").unwrap_or(&new_fm);
    Ok(format!("---\n{new_fm}---{body}"))
}

/// Recursively delete all `.md` files under `dir`. Returns count deleted.
fn delete_md_files_in_dir(dir: &Path) -> Result<usize, ZeniiError> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut count = 0usize;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let path = entry.path();
        if ft.is_dir() {
            count += delete_md_files_in_dir(&path)?;
        } else if ft.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            std::fs::remove_file(&path)?;
            count += 1;
        }
    }
    Ok(count)
}

fn walk_pages_dir(dir: &Path, pages: &mut Vec<WikiPage>) -> Result<(), ZeniiError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_pages_dir(&path, pages)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            let slug = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            pages.push(parse_page(slug, &path, None)?);
        }
    }
    Ok(())
}

fn find_page_in_dir(dir: &Path, slug: &str) -> Result<Option<WikiPage>, ZeniiError> {
    if !dir.exists() {
        return Ok(None);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_page_in_dir(&path, slug)? {
                return Ok(Some(found));
            }
        } else if path.extension().is_some_and(|e| e == "md") {
            let file_slug = path.file_stem().unwrap_or_default().to_string_lossy();
            if file_slug == slug {
                return Ok(Some(parse_page(slug.to_string(), &path, None)?));
            }
        }
    }
    Ok(None)
}

fn parse_page(
    slug: String,
    path: &Path,
    filename_hint: Option<&str>,
) -> Result<WikiPage, ZeniiError> {
    let content = std::fs::read_to_string(path)?;

    let (frontmatter_str, body) = if let Some(rest) = content.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            (&rest[..end_idx], &rest[end_idx + 4..])
        } else {
            ("", content.as_str())
        }
    } else {
        ("", content.as_str())
    };

    #[derive(serde::Deserialize, Default)]
    struct Frontmatter {
        title: Option<String>,
        #[serde(rename = "type", default)]
        page_type: Option<String>,
        #[serde(default)]
        tags: Option<Vec<String>>,
        #[serde(default)]
        sources: Option<Vec<String>>,
        #[serde(default)]
        updated: Option<String>,
    }

    let fm: Frontmatter = serde_yaml::from_str(frontmatter_str).unwrap_or_default();
    let tldr = extract_section(body, "TLDR");
    let wikilinks = extract_wikilinks(body);

    let title = fm.title.unwrap_or_else(|| {
        // 1. First # heading in the body
        body.lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l[2..].trim().to_string())
            // 2. Original filename (preserves casing: "GitHub Stars.md" → "GitHub Stars")
            .or_else(|| {
                filename_hint
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            })
            // 3. Humanize slug: "my-doc" → "My Doc"
            .unwrap_or_else(|| {
                slug.replace('-', " ")
                    .split_whitespace()
                    .map(|w| {
                        let mut chars = w.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().to_string() + chars.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
    });

    let type_from_dir = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|dir| match dir {
            "concepts" => "concept",
            "entities" => "entity",
            "comparisons" => "comparison",
            _ => "topic",
        });

    Ok(WikiPage {
        slug,
        title,
        page_type: fm
            .page_type
            .or_else(|| type_from_dir.map(String::from))
            .unwrap_or_else(|| "topic".into()),
        tags: fm.tags.unwrap_or_default(),
        sources: fm.sources.unwrap_or_default(),
        updated: fm.updated.unwrap_or_default(),
        tldr,
        body: body.to_string(),
        wikilinks,
    })
}

fn extract_section(body: &str, heading: &str) -> String {
    let target = format!("## {heading}");
    let mut in_section = false;
    let mut lines: Vec<&str> = Vec::new();
    for line in body.lines() {
        if line.starts_with("## ") {
            if in_section {
                break;
            }
            if line == target {
                in_section = true;
            }
        } else if in_section {
            lines.push(line);
        }
    }
    lines.join("\n").trim().to_string()
}

fn extract_wikilinks(body: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find("[[") {
        rest = &rest[open + 2..];
        if let Some(close) = rest.find("]]") {
            let link = rest[..close].to_string();
            if !link.is_empty() {
                links.push(link);
            }
            rest = &rest[close + 2..];
        } else {
            break;
        }
    }
    links
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn sample_page_content() -> &'static str {
        r#"---
title: "Test Concept"
type: concept
tags: [test, example]
sources: [source.md]
updated: 2026-01-01
---

## TLDR
This is a test concept.

## Body
References [[another-page]] for related info.

## See Also
- [[another-page]]
"#
    }

    fn write_page(dir: &Path, subdir: &str, slug: &str, content: &str) {
        let page_dir = dir.join("pages").join(subdir);
        fs::create_dir_all(&page_dir).unwrap();
        fs::write(page_dir.join(format!("{slug}.md")), content).unwrap();
    }

    // W1: WikiManager::new creates required subdirectory structure
    #[test]
    fn new_creates_pages_subdirs() {
        let dir = TempDir::new().unwrap();
        let _mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        assert!(dir.path().join("pages/concepts").is_dir());
    }

    // W2: Listing an empty wiki returns an empty vec
    #[test]
    fn list_pages_empty_wiki_returns_empty() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let pages = mgr.list_pages().unwrap();
        assert!(pages.is_empty());
    }

    // W3: list_pages discovers all .md files under pages/
    #[test]
    fn list_pages_finds_markdown_files() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "page-one", sample_page_content());
        write_page(dir.path(), "concepts", "page-two", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let pages = mgr.list_pages().unwrap();
        assert_eq!(pages.len(), 2);
    }

    // W4: list_pages ignores non-.md files
    #[test]
    fn list_pages_skips_non_markdown() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "real-page", sample_page_content());
        // Write a .txt sibling that should be ignored
        let concept_dir = dir.path().join("pages/concepts");
        fs::create_dir_all(&concept_dir).unwrap();
        fs::write(concept_dir.join("notes.txt"), "ignore me").unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let pages = mgr.list_pages().unwrap();
        assert_eq!(pages.len(), 1);
    }

    // W5: get_page returns Some with correct title for an existing slug
    #[test]
    fn get_page_returns_some_for_existing_slug() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "test-page", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let page = mgr.get_page("test-page").unwrap();
        assert!(page.is_some());
        assert_eq!(page.unwrap().title, "Test Concept");
    }

    // W6: get_page returns None for a slug that does not exist
    #[test]
    fn get_page_returns_none_for_missing_slug() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let page = mgr.get_page("no-such-page").unwrap();
        assert!(page.is_none());
    }

    // W7: search_pages matches against page title (case-insensitive)
    #[test]
    fn search_pages_matches_title() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "my-concept", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let results = mgr.search_pages("concept").unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|p| p.slug == "my-concept"));
    }

    // W8: search_pages matches against body text
    #[test]
    fn search_pages_matches_body_content() {
        let dir = TempDir::new().unwrap();
        let content = r#"---
title: "Body Search Test"
type: topic
tags: []
sources: []
updated: 2026-01-01
---

## TLDR
Short summary.

## Body
This page contains rare_unique_word somewhere in the body.
"#;
        write_page(dir.path(), "topics", "body-page", content);
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let results = mgr.search_pages("rare_unique_word").unwrap();
        assert!(!results.is_empty());
    }

    // W9: empty query string returns all pages
    #[test]
    fn search_pages_empty_query_returns_all() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "page-alpha", sample_page_content());
        write_page(dir.path(), "concepts", "page-beta", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let results = mgr.search_pages("").unwrap();
        assert_eq!(results.len(), 2);
    }

    // W10: frontmatter fields are extracted correctly
    #[test]
    fn parse_frontmatter_extracts_fields() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "fm-test", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let page = mgr.get_page("fm-test").unwrap().unwrap();
        assert_eq!(page.title, "Test Concept");
        assert_eq!(page.page_type, "concept");
        assert_eq!(page.tags, vec!["test", "example"]);
        assert_eq!(page.sources, vec!["source.md"]);
        assert_eq!(page.updated, "2026-01-01");
    }

    // W11: missing frontmatter fields fall back to safe defaults
    #[test]
    fn parse_frontmatter_partial_uses_defaults() {
        let dir = TempDir::new().unwrap();
        let minimal = r#"---
title: "Minimal Page"
---

## TLDR
Bare minimum.

## Body
No tags, no type, no sources.
"#;
        write_page(dir.path(), "topics", "minimal-page", minimal);
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let page = mgr.get_page("minimal-page").unwrap().unwrap();
        assert_eq!(page.page_type, "topic");
        assert!(page.tags.is_empty());
        assert!(page.sources.is_empty());
    }

    // W12: sync_to_memory stores one entry per page and returns the count
    #[tokio::test]
    async fn sync_to_memory_stores_one_entry_per_page() {
        use crate::memory::in_memory_store::InMemoryStore;

        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "page-1", sample_page_content());
        write_page(dir.path(), "concepts", "page-2", sample_page_content());
        write_page(dir.path(), "concepts", "page-3", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let store = InMemoryStore::new();
        let count = mgr.sync_to_memory(&store).await.unwrap();
        assert_eq!(count, 3);
    }

    // W13: graph nodes correspond 1-to-1 with pages
    #[test]
    fn graph_nodes_match_pages() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "node-a", sample_page_content());
        write_page(dir.path(), "concepts", "node-b", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let graph = mgr.graph().unwrap();
        assert_eq!(graph.nodes.len(), 2);
    }

    // W14: wikilinks in page body generate directed edges in the graph
    #[test]
    fn graph_edges_follow_wikilinks() {
        let dir = TempDir::new().unwrap();
        let page_a = r#"---
title: "Page A"
type: concept
tags: []
sources: []
updated: 2026-01-01
---

## TLDR
Links to page-b.

## Body
See [[page-b]] for details.
"#;
        let page_b = r#"---
title: "Page B"
type: concept
tags: []
sources: []
updated: 2026-01-01
---

## TLDR
The target page.

## Body
No outbound links here.
"#;
        write_page(dir.path(), "concepts", "page-a", page_a);
        write_page(dir.path(), "concepts", "page-b", page_b);
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let graph = mgr.graph().unwrap();
        let has_edge = graph
            .edges
            .iter()
            .any(|e| e.from == "page-a" && e.to == "page-b");
        assert!(has_edge, "expected edge from page-a to page-b");
    }

    // W15: ingest uses original filename as title when no frontmatter title present
    #[test]
    fn ingest_title_falls_back_to_filename() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let page = mgr
            .ingest("GitHub Stars Growth Tips.md", "No frontmatter here.\n")
            .unwrap();
        assert_eq!(page.title, "GitHub Stars Growth Tips");
    }

    // W16: ingest prefers # heading over filename when both are present
    #[test]
    fn ingest_title_prefers_heading_over_filename() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let content = "# Actual Heading Title\n\nSome body text.\n";
        let page = mgr.ingest("some-filename.md", content).unwrap();
        assert_eq!(page.title, "Actual Heading Title");
    }

    // W17: ingest prefers frontmatter title over # heading and filename
    #[test]
    fn ingest_title_prefers_frontmatter_over_heading_and_filename() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let content = "---\ntitle: \"Frontmatter Wins\"\n---\n\n# Heading Title\n\nBody.\n";
        let page = mgr.ingest("some-filename.md", content).unwrap();
        assert_eq!(page.title, "Frontmatter Wins");
    }

    // W18: title falls back to humanized slug when no frontmatter, heading, or filename hint
    #[test]
    fn parse_page_title_falls_back_to_humanized_slug() {
        let dir = TempDir::new().unwrap();
        write_page(
            dir.path(),
            "topics",
            "my-doc-slug",
            "No frontmatter, no heading.\n",
        );
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let page = mgr.get_page("my-doc-slug").unwrap().unwrap();
        assert_eq!(page.title, "My Doc Slug");
    }

    // W19: write_page creates the file in the correct subdirectory and parses it back
    #[test]
    fn write_page_creates_file_in_correct_subdir() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let page = mgr
            .write_page("concepts", "test-concept", sample_page_content())
            .unwrap();
        assert_eq!(page.slug, "test-concept");
        assert!(dir.path().join("pages/concepts/test-concept.md").exists());
    }

    // W20: write_page returns a Validation error for an unrecognised page type
    #[test]
    fn write_page_rejects_invalid_type() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let err = mgr
            .write_page("invalid-type", "my-page", "content")
            .unwrap_err();
        assert!(matches!(err, crate::error::ZeniiError::Validation(_)));
    }

    // W21: update_index rewrites index.md with a line per page
    #[test]
    fn update_index_writes_entries_for_all_pages() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "page-a", sample_page_content());
        write_page(dir.path(), "topics", "page-b", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        mgr.update_index().unwrap();
        let index = fs::read_to_string(dir.path().join("index.md")).unwrap();
        assert!(index.contains("[[page-a]]"), "index must mention page-a");
        assert!(index.contains("[[page-b]]"), "index must mention page-b");
    }

    // W22: append_log appends entries to log.md without overwriting existing content
    #[test]
    fn append_log_appends_without_overwriting() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("log.md"), "# Wiki Log\n").unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        mgr.append_log("## [2026-04-09] ingest | first-entry").unwrap();
        mgr.append_log("## [2026-04-09] ingest | second-entry").unwrap();
        let log = fs::read_to_string(dir.path().join("log.md")).unwrap();
        assert!(log.contains("# Wiki Log"), "initial content must be preserved");
        assert!(log.contains("first-entry"), "first entry must be present");
        assert!(log.contains("second-entry"), "second entry must be present");
    }

    // W23: save_source writes raw content to wiki/sources/
    #[test]
    fn save_source_writes_to_sources_dir() {
        let dir = TempDir::new().unwrap();
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        mgr.save_source("my-doc.md", "Raw source content").unwrap();
        let saved = fs::read_to_string(dir.path().join("sources/my-doc.md")).unwrap();
        assert_eq!(saved, "Raw source content");
    }

    // W24: new() creates sources/ directory
    #[test]
    fn new_creates_sources_dir() {
        let dir = TempDir::new().unwrap();
        let _mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        assert!(dir.path().join("sources").is_dir(), "sources/ must be created");
    }

    // W25: new() seeds SCHEMA.md from embedded template
    #[test]
    fn new_seeds_schema_md() {
        let dir = TempDir::new().unwrap();
        let _mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let schema = fs::read_to_string(dir.path().join("SCHEMA.md")).unwrap();
        assert!(!schema.is_empty(), "SCHEMA.md must be seeded with content");
    }

    // W26: new() does not overwrite existing SCHEMA.md
    #[test]
    fn new_does_not_overwrite_existing_schema() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("SCHEMA.md"), "custom content").unwrap();
        let _mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let schema = fs::read_to_string(dir.path().join("SCHEMA.md")).unwrap();
        assert_eq!(schema, "custom content", "existing SCHEMA.md must not be overwritten");
    }

    // W27: new() creates stub index.md on first boot
    #[test]
    fn new_creates_stub_index_md() {
        let dir = TempDir::new().unwrap();
        let _mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        assert!(dir.path().join("index.md").exists(), "index.md must be created");
    }

    // W28: new() creates stub log.md on first boot
    #[test]
    fn new_creates_stub_log_md() {
        let dir = TempDir::new().unwrap();
        let _mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        assert!(dir.path().join("log.md").exists(), "log.md must be created");
    }

    fn entity_page_content() -> &'static str {
        "---\ntitle: \"Test Entity\"\ntype: entity\ntags: [person]\nsources: [source.md]\nupdated: 2026-01-01\n---\n\n## TLDR\nThis is a test entity.\n\n## Body\nAn entity page.\n"
    }

    // W29: update_index() writes typed sections in SCHEMA order (Concepts before Entities)
    #[test]
    fn update_index_writes_typed_sections_in_order() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "my-concept", sample_page_content());
        write_page(dir.path(), "entities", "some-entity", entity_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        mgr.update_index().unwrap();
        let index = fs::read_to_string(dir.path().join("index.md")).unwrap();
        assert!(index.contains("## Concepts"), "must have ## Concepts section");
        assert!(index.contains("## Entities"), "must have ## Entities section");
        let concept_pos = index.find("## Concepts").unwrap();
        let entity_pos = index.find("## Entities").unwrap();
        assert!(concept_pos < entity_pos, "Concepts section must precede Entities");
    }

    // W30: update_index() omits sections that have no pages
    #[test]
    fn update_index_omits_empty_sections() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "only-concept", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        mgr.update_index().unwrap();
        let index = fs::read_to_string(dir.path().join("index.md")).unwrap();
        assert!(index.contains("## Concepts"), "Concepts section must be present");
        assert!(!index.contains("## Entities"), "empty Entities section must be omitted");
    }

    // W31: update_index() entries do not have trailing (type) annotation
    #[test]
    fn update_index_entries_have_no_type_annotation() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "test-concept", sample_page_content());
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        mgr.update_index().unwrap();
        let index = fs::read_to_string(dir.path().join("index.md")).unwrap();
        assert!(
            !index.contains("(concept)"),
            "entries must not have trailing (type) annotation"
        );
    }

    // W32: lint() reports broken wikilinks
    #[test]
    fn lint_detects_broken_wikilinks() {
        let dir = TempDir::new().unwrap();
        let content = "---\ntitle: \"Page With Broken Link\"\ntype: concept\ntags: []\nsources: []\nupdated: 2026-01-01\n---\n\n## TLDR\nLinks to nothing.\n\n## Body\nSee [[nonexistent-page]] for details.\n";
        write_page(dir.path(), "concepts", "page-with-broken-link", content);
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let issues = mgr.lint().unwrap();
        assert!(
            issues.iter().any(|i| i.kind == "broken_wikilink"),
            "broken wikilink must be reported"
        );
    }

    // W33: lint() reports orphan pages (multiple pages, none cross-linking)
    #[test]
    fn lint_detects_orphan_pages() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "page-a", sample_page_content());
        write_page(dir.path(), "concepts", "page-b", sample_page_content());
        // sample_page_content has [[another-page]] which doesn't exist — orphan check is
        // about inbound links; both pages have 0 inbound links from each other
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let issues = mgr.lint().unwrap();
        assert!(
            issues.iter().any(|i| i.kind == "orphan_page"),
            "orphan pages must be reported"
        );
    }

    // W34: lint() reports pages that are not listed in index.md
    #[test]
    fn lint_detects_missing_index_entries() {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "concepts", "unlisted-page", sample_page_content());
        // index.md is the stub created by new() — does not list unlisted-page
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let issues = mgr.lint().unwrap();
        assert!(
            issues.iter().any(|i| i.kind == "missing_index_entry"),
            "missing index entry must be reported"
        );
    }

    // W35: lint() reports pages with an empty 'updated' frontmatter field
    #[test]
    fn lint_detects_missing_updated_field() {
        let dir = TempDir::new().unwrap();
        let content = "---\ntitle: \"No Date\"\ntype: concept\ntags: []\nsources: []\n---\n\n## TLDR\nNo date.\n\n## Body\nContent.\n";
        write_page(dir.path(), "concepts", "no-date-page", content);
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let issues = mgr.lint().unwrap();
        assert!(
            issues.iter().any(|i| i.kind == "missing_updated"),
            "missing updated field must be reported"
        );
    }

    // W36: lint() reports no broken_wikilink or missing_updated for a compliant cross-linked wiki
    #[test]
    fn lint_returns_clean_for_compliant_cross_linked_wiki() {
        let dir = TempDir::new().unwrap();
        let page_a = "---\ntitle: \"Page A\"\ntype: concept\ntags: []\nsources: []\nupdated: 2026-01-01\n---\n\n## TLDR\nA.\n\n## Body\nSee [[page-b]].\n";
        let page_b = "---\ntitle: \"Page B\"\ntype: concept\ntags: []\nsources: []\nupdated: 2026-01-01\n---\n\n## TLDR\nB.\n\n## Body\nSee [[page-a]].\n";
        write_page(dir.path(), "concepts", "page-a", page_a);
        write_page(dir.path(), "concepts", "page-b", page_b);
        let mgr = WikiManager::new(dir.path().to_path_buf()).unwrap();
        mgr.update_index().unwrap();
        let issues = mgr.lint().unwrap();
        assert!(
            !issues.iter().any(|i| i.kind == "broken_wikilink"),
            "no broken wikilinks expected"
        );
        assert!(
            !issues.iter().any(|i| i.kind == "missing_updated"),
            "no missing updated expected"
        );
        assert!(
            !issues.iter().any(|i| i.kind == "missing_index_entry"),
            "no missing index entries expected"
        );
    }

    #[test]
    fn test_set_prompt_writes_file() {
        let dir = tempfile::tempdir().unwrap();
        let wm = WikiManager::new(dir.path().to_path_buf()).unwrap();
        wm.set_prompt("Custom prompt content").unwrap();
        let content = std::fs::read_to_string(dir.path().join("INGEST_PROMPT.md")).unwrap();
        assert_eq!(content, "Custom prompt content");
    }

    #[test]
    fn test_set_prompt_rejects_empty() {
        let dir = tempfile::tempdir().unwrap();
        let wm = WikiManager::new(dir.path().to_path_buf()).unwrap();
        assert!(wm.set_prompt("").is_err());
    }

    #[test]
    fn test_set_prompt_rejects_too_long() {
        let dir = tempfile::tempdir().unwrap();
        let wm = WikiManager::new(dir.path().to_path_buf()).unwrap();
        let long = "x".repeat(4001);
        assert!(wm.set_prompt(&long).is_err());
    }

    #[test]
    fn test_delete_all_pages_removes_md_files() {
        let dir = tempfile::tempdir().unwrap();
        let wm = WikiManager::new(dir.path().to_path_buf()).unwrap();
        // Write a page file
        let page_path = dir.path().join("pages").join("concepts").join("test.md");
        std::fs::create_dir_all(page_path.parent().unwrap()).unwrap();
        std::fs::write(&page_path, "# Test\npage_type: concept\n").unwrap();
        let count = wm.delete_all_pages().unwrap();
        assert_eq!(count, 1);
        assert!(!page_path.exists());
    }

    #[test]
    fn test_delete_all_pages_resets_index() {
        let dir = tempfile::tempdir().unwrap();
        let wm = WikiManager::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("index.md"), "# Wiki Index\n- [[foo]]\n").unwrap();
        wm.delete_all_pages().unwrap();
        let index = std::fs::read_to_string(dir.path().join("index.md")).unwrap();
        assert!(index.contains("No pages yet"));
    }
}
