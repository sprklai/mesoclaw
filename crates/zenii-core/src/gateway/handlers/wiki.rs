use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::error::ZeniiError;
use crate::gateway::state::AppState;
use crate::wiki::{FixedIssue, WikiPage};

// ── Request / query types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

/// L4: pagination query params for GET /wiki
#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Deserialize, Default)]
pub struct LintRequest {
    pub auto_fix: Option<bool>,
}

#[derive(Deserialize)]
pub struct IngestRequest {
    pub content: String,
    pub filename: String,
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub question: String,
    pub save: Option<bool>,
    pub model: Option<String>,
}

// ── Response types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct IngestResponse {
    primary_slug: String,
    pages: Vec<WikiPage>,
    message: String,
}

/// Page definition as returned by the LLM in JSON.
#[derive(Deserialize, Clone)]
struct LlmPage {
    page_type: String,
    slug: String,
    content: String,
}

#[derive(Serialize)]
struct QueryResponse {
    answer: String,
    citations: Vec<String>,
    saved_page: Option<WikiPage>,
}

#[derive(Serialize)]
struct LintResponse {
    issues: Vec<crate::wiki::LintIssue>,
    fixed: Vec<FixedIssue>,
    summary: String,
}

#[derive(Deserialize)]
pub struct RegenerateRequest {
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct SetPromptRequest {
    pub content: String,
}

#[derive(Serialize)]
struct PromptResponse {
    content: String,
}

#[derive(Serialize)]
struct DeletePagesResponse {
    deleted: usize,
    message: String,
}

#[derive(Serialize)]
struct DeleteSourcesResponse {
    deleted: usize,
    message: String,
}

#[derive(Serialize)]
struct SourceListItem {
    filename: String,
    hash: String,
    active: bool,
    last_run_id: Option<String>,
    pages: Vec<String>, // slugs of pages derived from this source
}

#[derive(Serialize)]
struct RegenerateResponse {
    sources_processed: usize,
    pages_generated: usize,
    message: String,
}

#[derive(Serialize)]
struct DeleteSourceResponse {
    filename: String,
    deleted_pages: Vec<String>,
    rebuilt_pages: Vec<String>,
    message: String,
}

#[derive(Deserialize)]
pub struct DeleteSourceQuery {
    pub model: Option<String>,
}

// ── Query page-selection constants ───────────────────────────────────────────

/// Maximum number of pages to include as full bodies in a query prompt.
const QUERY_MAX_PAGES: usize = 15;
/// Minimum keyword-hit score for a page to be included in query context.
const QUERY_MIN_SCORE: u32 = 1;

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /wiki?limit=50&offset=0 — list wiki pages with pagination (L4).
pub async fn list_wiki_pages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let result = tokio::task::spawn_blocking(move || wiki.list_pages()).await;
    match result {
        Ok(Ok(pages)) => {
            let total = pages.len();
            let page_slice: Vec<_> = pages.into_iter().skip(offset).take(limit).collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "pages": page_slice,
                    "total": total,
                    "limit": limit,
                    "offset": offset,
                })),
            )
                .into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// GET /wiki/{slug} — fetch a single wiki page by slug.
pub async fn get_wiki_page(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let result = tokio::task::spawn_blocking(move || wiki.get_page(&slug)).await;
    match result {
        Ok(Ok(Some(page))) => (StatusCode::OK, Json(serde_json::json!(page))).into_response(),
        Ok(Ok(None)) => StatusCode::NOT_FOUND.into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// GET /wiki/search?q= — full-text search over wiki pages.
pub async fn search_wiki_pages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = params.q.unwrap_or_default();
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let result = tokio::task::spawn_blocking(move || wiki.search_pages(&q)).await;
    match result {
        Ok(Ok(pages)) => (StatusCode::OK, Json(serde_json::json!(pages))).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// POST /wiki/ingest — ingest a raw source document into the wiki.
///
/// Calls the configured AI agent to synthesize multiple typed wiki pages (concepts, entities,
/// topics, comparisons, queries) from the source. Falls back to a single topic page when no
/// AI model is configured or the LLM response cannot be parsed.
///
/// Uses staged builds (write to unique tempdir first, swap on success) and updates the manifest
/// so future regenerate/delete operations have accurate source lineage.
pub async fn ingest_wiki_source(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IngestRequest>,
) -> impl IntoResponse {
    use crate::wiki::{PageRecord, RunRecord, SourceRecord, WikiManager};

    let log_max_lines = state.config.load().wiki_log_max_lines;

    // ── Size guard: reject source content that exceeds the configured limit ──
    {
        let max_bytes = state.config.load().wiki_max_source_size_mb * 1024 * 1024;
        if body.content.len() as u64 > max_bytes {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!(
                        "source content too large ({} bytes); limit is {} MiB",
                        body.content.len(),
                        state.config.load().wiki_max_source_size_mb
                    )
                })),
            )
                .into_response();
        }
    }

    // ── Step 1: Save the raw source (blocking I/O, holds mutex) ─────────────
    {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        let filename = body.filename.clone();
        let content = body.content.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || wiki.save_source(&filename, &content))
            .await
            .unwrap_or_else(|e| Err(crate::error::ZeniiError::Gateway(e.to_string())))
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to save source: {e}")})),
            )
                .into_response();
        }
    }

    let run_id = WikiManager::new_run_id();
    let source_hash = WikiManager::hash_content(&body.content);

    // ── Step 2: Read prompts + schema for hashing (blocking, holds mutex) ──────
    // Also retains the schema text so run_compiler can receive it without re-reading (M4 fix).
    let (prompt_hash, schema_hash, schema_text) = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || {
            let prompt = wiki.read_ingest_prompt().unwrap_or_default();
            let schema =
                std::fs::read_to_string(wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default();
            (
                WikiManager::hash_content(&prompt),
                WikiManager::hash_content(&schema),
                schema,
            )
        })
        .await
        .unwrap_or_else(|_| (String::new(), String::new(), String::new()))
    };

    // ── Step 3: LLM compiler (async, no mutex held) ──────────────────────────
    let llm_result = run_compiler(
        &state,
        &[(body.filename.clone(), body.content.clone())],
        body.model.as_deref(),
        &schema_text,
    )
    .await;

    // ── Step 4: Staged build + manifest update (blocking, holds mutex) ───────
    // run_compiler returns (LlmPage, source_filename) pairs; for a single-source ingest
    // the source is always body.filename so we just extract the page.
    let (pages, used_llm) = if let Ok(llm_pages_with_source) = llm_result {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        let llm_pages_c: Vec<LlmPage> = llm_pages_with_source.into_iter().map(|(p, _)| p).collect();
        tokio::task::spawn_blocking(move || match wiki.begin_staged_build() {
            Ok(rebuild_dir) => {
                let mut staged = Vec::new();
                for p in &llm_pages_c {
                    if wiki
                        .write_staged_page(&rebuild_dir, &p.page_type, &p.slug, &p.content)
                        .is_ok()
                    {
                        staged.push(p.clone());
                    }
                }
                match wiki.commit_staged_build(&rebuild_dir) {
                    Ok(_) => {
                        let mut written = Vec::new();
                        for p in staged {
                            if let Ok(page) = wiki.write_page(&p.page_type, &p.slug, &p.content) {
                                written.push(page);
                            }
                        }
                        (written, true)
                    }
                    Err(e) => {
                        tracing::warn!("staged build commit failed: {e}");
                        wiki.abort_staged_build(&rebuild_dir);
                        (Vec::new(), false)
                    }
                }
            }
            Err(_) => (Vec::new(), false),
        })
        .await
        .unwrap_or((Vec::new(), false))
    } else {
        (Vec::new(), false)
    };

    if used_llm && !pages.is_empty() {
        // ── Step 5: Manifest + index update (blocking, holds mutex) ──────────
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        let filename = body.filename.clone();
        let pages_c = pages.clone();
        let run_id_c = run_id.clone();
        let source_hash_c = source_hash.clone();
        let model_c = body.model.clone();
        let prompt_hash_c = prompt_hash.clone();
        let schema_hash_c = schema_hash.clone();
        tokio::task::spawn_blocking(move || {
            let manifest = wiki.read_manifest_or_bootstrap();
            let (mut sources, mut page_records) = match manifest {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        "manifest read failed after ingest, skipping manifest update: {e}"
                    );
                    return;
                }
            };
            sources.retain(|s| s.filename != filename);
            sources.push(SourceRecord {
                filename: filename.clone(),
                hash: source_hash_c,
                active: true,
                last_run_id: Some(run_id_c.clone()),
                source_type: "text".to_string(),
            });
            for page in &pages_c {
                page_records.retain(|r| r.slug != page.slug);
                page_records.push(PageRecord {
                    slug: page.slug.clone(),
                    page_type: page.page_type.clone(),
                    path: format!("pages/{}/{}.md", page.page_type, page.slug),
                    sources: vec![filename.clone()],
                    last_run_id: run_id_c.clone(),
                    managed_by: "source_ingest".to_string(),
                });
            }
            if let Err(e) = wiki.write_manifest(&sources, &page_records) {
                tracing::warn!("manifest write failed after ingest: {e}");
            }
            let _ = wiki.append_run(&RunRecord {
                run_id: run_id_c,
                timestamp: chrono::Utc::now().to_rfc3339(),
                model: model_c,
                prompt_hash: prompt_hash_c,
                schema_hash: schema_hash_c,
                sources: vec![filename.clone()],
                status: "success".to_string(),
                pages_written: pages_c.iter().map(|p| p.slug.clone()).collect(),
            });
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Err(e) = wiki.update_index() {
                tracing::error!("wiki index update failed after ingest: {e}");
            }
            if let Err(e) = wiki.append_log_with_limit(
                &format!(
                    "## [{date}] ingest | {filename} — {} page(s) generated",
                    pages_c.len()
                ),
                log_max_lines,
            ) {
                tracing::warn!("wiki log append failed: {e}");
            }
        })
        .await
        .ok();

        if let Err(e) = state
            .wiki
            .lock()
            .await
            .sync_to_memory(state.memory.as_ref())
            .await
        {
            tracing::warn!("wiki memory sync failed: {e}");
        }
        let primary_slug = pages.first().map(|p| p.slug.clone()).unwrap_or_default();
        return (
            StatusCode::OK,
            Json(serde_json::json!(IngestResponse {
                primary_slug,
                message: format!("{} page(s) generated from '{}'", pages.len(), body.filename),
                pages,
            })),
        )
            .into_response();
    }

    // ── Fallback: write raw content as a single topic page (blocking, holds mutex) ──
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let filename = body.filename.clone();
    let content = body.content.clone();
    let source_hash_fb = WikiManager::hash_content(&body.content);
    // Returns (page_written, index_error): page may be Some even when index update fails.
    let fallback_result = tokio::task::spawn_blocking(move || {
        let page = wiki.ingest(&filename, &content)?;
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let (mut sources, page_records) = wiki.read_manifest_or_bootstrap()?;
        sources.retain(|s| s.filename != filename);
        sources.push(crate::wiki::SourceRecord {
            filename: filename.clone(),
            hash: source_hash_fb,
            active: true,
            last_run_id: None,
            source_type: "text".to_string(),
        });
        if let Err(e) = wiki.write_manifest(&sources, &page_records) {
            tracing::warn!("manifest write failed after fallback ingest: {e}");
        }
        let index_err = wiki.update_index().err();
        if let Err(e) = wiki.append_log_with_limit(
            &format!("## [{date}] ingest | {filename} — fallback single-page (no LLM)"),
            log_max_lines,
        ) {
            tracing::warn!("wiki log append failed: {e}");
        }
        Ok::<_, crate::error::ZeniiError>((page, index_err))
    })
    .await;

    match fallback_result {
        Ok(Ok((page, None))) => {
            if let Err(e) = state
                .wiki
                .lock()
                .await
                .sync_to_memory(state.memory.as_ref())
                .await
            {
                tracing::warn!("wiki memory sync failed: {e}");
            }
            let primary_slug = page.slug.clone();
            (
                StatusCode::OK,
                Json(serde_json::json!(IngestResponse {
                    primary_slug,
                    message: format!(
                        "1 page created from '{}' (LLM unavailable — configure a provider for full wiki generation)",
                        body.filename
                    ),
                    pages: vec![page],
                })),
            )
                .into_response()
        }
        Ok(Ok((page, Some(index_err)))) => {
            tracing::error!("wiki index update failed after fallback ingest: {index_err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": index_err.to_string(),
                    "pages_written": [page.slug],
                })),
            )
                .into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// GET /wiki/sources — list all ingested source files with manifest metadata.
pub async fn list_wiki_sources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let result = tokio::task::spawn_blocking(move || {
        let sources = wiki.list_sources()?;
        let (_, page_records) = wiki.read_manifest_or_bootstrap()?;
        let items: Vec<SourceListItem> = sources
            .into_iter()
            .map(|s| {
                let pages = page_records
                    .iter()
                    .filter(|p| p.sources.contains(&s.filename))
                    .map(|p| p.slug.clone())
                    .collect();
                SourceListItem {
                    filename: s.filename,
                    hash: s.hash,
                    active: s.active,
                    last_run_id: s.last_run_id,
                    pages,
                }
            })
            .collect();
        Ok::<_, crate::error::ZeniiError>(items)
    })
    .await;
    match result {
        Ok(Ok(items)) => (StatusCode::OK, Json(serde_json::json!(items))).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// GET /wiki/dir — return the absolute path to the wiki sources directory (for the Open Folder button).
pub async fn get_wiki_dir(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let path = tokio::task::spawn_blocking(move || {
        wiki.wiki_dir()
            .join("sources")
            .to_string_lossy()
            .into_owned()
    })
    .await
    .unwrap_or_default();
    (StatusCode::OK, Json(serde_json::json!({"path": path}))).into_response()
}

/// POST /wiki/regenerate — clear all source-generated pages and recompile from all sources.
///
/// Preserves `user_query` pages. Uses staged builds for atomicity: pages are never
/// deleted until after the LLM compile succeeds and the staged build is committed.
pub async fn regenerate_wiki(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegenerateRequest>,
) -> impl IntoResponse {
    use crate::wiki::{PageRecord, RunRecord, SourceRecord, WikiManager};
    use std::collections::HashMap;

    let log_max_lines = state.config.load().wiki_log_max_lines;

    // ── Step 1: Read manifest + collect sources (blocking) ─────────────────────
    // NOTE: live pages are NOT deleted here — deletion happens inside Step 3 only
    // after the staged build is committed (H3 fix: atomic swap, no pre-delete).
    struct PrepResult {
        sources_list: Vec<(String, String)>,
        manifest_pages: Vec<PageRecord>,
    }
    let prep = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || -> Result<PrepResult, crate::error::ZeniiError> {
            let (manifest_sources, manifest_pages) = wiki.read_manifest_or_bootstrap()?;
            let sources_list: Vec<(String, String)> = if manifest_sources.is_empty() {
                wiki.list_sources()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|r| wiki.read_source(&r.filename).ok().map(|c| (r.filename, c)))
                    .collect()
            } else {
                manifest_sources
                    .iter()
                    .filter(|r| r.active)
                    .filter_map(|r| {
                        wiki.read_source(&r.filename)
                            .ok()
                            .map(|c| (r.filename.clone(), c))
                    })
                    .collect()
            };
            Ok(PrepResult {
                sources_list,
                manifest_pages,
            })
        })
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("wiki regenerate prep task panicked: {e}");
            Err(crate::error::ZeniiError::Gateway(
                "prep task panicked".into(),
            ))
        })
        .unwrap_or_else(|e| {
            tracing::warn!("wiki regenerate manifest read failed: {e}");
            PrepResult {
                sources_list: Vec::new(),
                manifest_pages: Vec::new(),
            }
        })
    };

    if prep.sources_list.is_empty() {
        return (
            StatusCode::OK,
            Json(serde_json::json!(RegenerateResponse {
                sources_processed: 0,
                pages_generated: 0,
                message: "No sources to regenerate from.".to_string(),
            })),
        )
            .into_response();
    }

    // ── Step 1b: Read SCHEMA.md once before the LLM loop (M4 fix) ──────────────
    // run_compiler iterates over N sources; reading SCHEMA.md inside would cost N disk reads.
    // We read it here and pass it in so it is read exactly once per regenerate call.
    let schema_text = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default()
        })
        .await
        .unwrap_or_default()
    };

    // ── Step 2: LLM compiler (async, no mutex held) ──────────────────────────
    // Returns (LlmPage, source_filename) pairs for correct provenance tracking (C6 fix).
    let llm_result = run_compiler(
        &state,
        &prep.sources_list,
        body.model.as_deref(),
        &schema_text,
    )
    .await;
    let llm_pages_with_source = match llm_result {
        Ok(p) if !p.is_empty() => p,
        result => {
            let reason = result
                .err()
                .unwrap_or_else(|| "LLM returned no pages.".to_string());
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let wiki = Arc::clone(&state.wiki).lock_owned().await;
            let reason_c = reason.clone();
            let _ = tokio::task::spawn_blocking(move || {
                wiki.append_log_with_limit(
                    &format!("## [{date}] regenerate | failed — {reason_c}"),
                    log_max_lines,
                )
            })
            .await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": reason})),
            )
                .into_response();
        }
    };

    // ── Step 3: Staged build + atomic swap + manifest (blocking, holds mutex) ──
    // Dedup slugs: first-write-wins policy across sources (C6 fix).
    let sources_list_c = prep.sources_list.clone();
    let manifest_pages_c = prep.manifest_pages;
    let model_c = body.model.clone();
    let llm_pages_with_source_c = llm_pages_with_source.clone();
    let schema_text_c = schema_text.clone();
    let write_result = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || {
            let rebuild_dir = wiki.begin_staged_build()?;

            // Build dedup map: slug → source_filename (first-write-wins, C6 fix)
            let mut slug_to_source: HashMap<String, String> = HashMap::new();
            for (p, source_filename) in &llm_pages_with_source_c {
                if let Some(existing_source) = slug_to_source.get(&p.slug) {
                    tracing::warn!(
                        slug = %p.slug,
                        first_source = %existing_source,
                        duplicate_source = %source_filename,
                        "duplicate slug from multiple sources — skipping duplicate (first-write-wins)"
                    );
                    continue;
                }
                slug_to_source.insert(p.slug.clone(), source_filename.clone());
                if let Err(e) = wiki.write_staged_page(&rebuild_dir, &p.page_type, &p.slug, &p.content) {
                    tracing::warn!("skipping page '{}': {e}", p.slug);
                    slug_to_source.remove(&p.slug);
                }
            }

            let committed = match wiki.commit_staged_build(&rebuild_dir) {
                Ok(c) => c,
                Err(e) => {
                    wiki.abort_staged_build(&rebuild_dir);
                    return Err(e);
                }
            };

            // Delete old managed pages only AFTER successful commit (H3 fix: atomic swap)
            let old_managed_pages: Vec<PageRecord> = manifest_pages_c
                .iter()
                .filter(|r| r.managed_by == "source_ingest")
                .cloned()
                .collect();
            if let Err(e) = wiki.delete_page_files(&old_managed_pages) {
                tracing::warn!("failed to delete old managed pages after commit: {e}");
            }

            let run_id = WikiManager::new_run_id();
            let prompt_hash = WikiManager::hash_content(&wiki.read_ingest_prompt().unwrap_or_default());
            // Use the schema already read before the compiler loop (M4 fix: no extra disk read).
            let schema_hash = WikiManager::hash_content(&schema_text_c);

            let mut written_pages = Vec::new();
            let mut new_page_records: Vec<PageRecord> = Vec::new();
            for (ptype, slug) in &committed {
                // Find the (page, source) pair for this slug
                if let Some((lp, src)) = llm_pages_with_source_c.iter().find(|(p, _)| &p.slug == slug)
                    && let Ok(page) = wiki.write_page(ptype, slug, &lp.content)
                {
                    written_pages.push(page);
                    // Use the actual source that produced this page (C6 fix: correct provenance)
                    new_page_records.push(PageRecord {
                        slug: slug.clone(),
                        page_type: ptype.clone(),
                        path: format!("pages/{ptype}/{slug}.md"),
                        sources: vec![src.clone()],
                        last_run_id: run_id.clone(),
                        managed_by: "source_ingest".to_string(),
                    });
                }
            }

            let new_source_records: Vec<SourceRecord> = sources_list_c
                .iter()
                .map(|(filename, content)| SourceRecord {
                    filename: filename.clone(),
                    hash: WikiManager::hash_content(content),
                    active: true,
                    last_run_id: Some(run_id.clone()),
                    source_type: "text".to_string(),
                })
                .collect();
            let preserved_query_pages: Vec<PageRecord> = manifest_pages_c
                .into_iter()
                .filter(|r| r.managed_by != "source_ingest")
                .collect();
            let all_page_records: Vec<PageRecord> =
                new_page_records.into_iter().chain(preserved_query_pages).collect();

            if let Err(e) = wiki.write_manifest(&new_source_records, &all_page_records) {
                tracing::warn!("manifest write failed after regenerate: {e}");
            }
            let _ = wiki.append_run(&RunRecord {
                run_id,
                timestamp: chrono::Utc::now().to_rfc3339(),
                model: model_c,
                prompt_hash,
                schema_hash,
                sources: sources_list_c.iter().map(|(f, _)| f.clone()).collect(),
                status: "success".to_string(),
                pages_written: written_pages.iter().map(|p| p.slug.clone()).collect(),
            });
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Err(e) = wiki.update_index() {
                tracing::warn!("wiki index update failed after regenerate: {e}");
            }
            if let Err(e) = wiki.append_log_with_limit(&format!(
                "## [{date}] regenerate | {} source(s) → {} page(s)",
                sources_list_c.len(),
                written_pages.len()
            ), log_max_lines) {
                tracing::warn!("wiki log append failed: {e}");
            }
            Ok::<_, crate::error::ZeniiError>(written_pages)
        })
        .await
    };

    let written_pages = match write_result {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("task panic: {e}")})),
            )
                .into_response();
        }
    };

    if let Err(e) = state
        .wiki
        .lock()
        .await
        .sync_to_memory(state.memory.as_ref())
        .await
    {
        tracing::warn!("wiki memory sync failed: {e}");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(RegenerateResponse {
            sources_processed: prep.sources_list.len(),
            pages_generated: written_pages.len(),
            message: format!(
                "Regenerated from {} source(s); {} pages written.",
                prep.sources_list.len(),
                written_pages.len()
            ),
        })),
    )
        .into_response()
}

/// POST /wiki/sources/:filename/regenerate — re-run the LLM compiler over a single source.
///
/// Recompiles from just this one source. Pages derived from this source are replaced
/// atomically: the LLM must produce pages successfully before any live files are removed
/// (H3 fix: no pre-delete). Shared pages (those that list multiple contributing sources)
/// are deleted and rebuilt from this source only — run a full regeneration to restore
/// multi-source synthesis.
pub async fn regenerate_wiki_source(
    Path(filename): Path<String>,
    State(state): State<Arc<AppState>>,
    body: Option<Json<RegenerateRequest>>,
) -> impl IntoResponse {
    use crate::wiki::{PageRecord, RunRecord, SourceRecord, WikiManager};

    let log_max_lines = state.config.load().wiki_log_max_lines;
    let model = body
        .as_ref()
        .and_then(|b| b.model.as_deref().map(String::from));

    // ── Step 1: Verify source, read manifest (blocking) ─────────────────────────
    // NOTE: live pages are NOT deleted here — deletion happens inside Step 3 only
    // after the staged build is committed (H3 fix: atomic swap, no pre-delete).
    struct SourcePrep {
        source_content: String,
        manifest_sources: Vec<crate::wiki::SourceRecord>,
        manifest_pages: Vec<PageRecord>,
        source_pages: Vec<PageRecord>,
    }
    let prep = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        let filename_c = filename.clone();
        tokio::task::spawn_blocking(move || {
            let source_content = wiki.read_source(&filename_c)?;
            let (manifest_sources, manifest_pages) = wiki.read_manifest_or_bootstrap()?;
            let source_pages: Vec<PageRecord> = manifest_pages
                .iter()
                .filter(|p| p.sources.contains(&filename_c) && p.managed_by == "source_ingest")
                .cloned()
                .collect();
            Ok::<_, crate::error::ZeniiError>(SourcePrep {
                source_content,
                manifest_sources,
                manifest_pages,
                source_pages,
            })
        })
        .await
    };

    let prep = match prep {
        Ok(Ok(p)) => p,
        Ok(Err(crate::error::ZeniiError::Validation(_))) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("source '{}' not found", filename)})),
            )
                .into_response();
        }
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("task panic: {e}")})),
            )
                .into_response();
        }
    };

    // ── Step 1b: Read SCHEMA.md once before the compiler (M4 fix) ──────────────
    // Avoids re-reading on every run_compiler call; passed in to avoid N disk reads.
    let schema_text = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default()
        })
        .await
        .unwrap_or_default()
    };

    // ── Step 2: LLM compiler (async, no mutex held) ──────────────────────────
    // Returns (LlmPage, source_filename) pairs. Source is always `filename` here
    // since we compile a single source.
    let sources_list = vec![(filename.clone(), prep.source_content.clone())];
    let llm_result = run_compiler(&state, &sources_list, model.as_deref(), &schema_text).await;
    let llm_pages_with_source = match llm_result {
        Ok(p) if !p.is_empty() => p,
        result => {
            let reason = result
                .err()
                .unwrap_or_else(|| "LLM returned no pages.".to_string());
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let wiki = Arc::clone(&state.wiki).lock_owned().await;
            let filename_c = filename.clone();
            let reason_c = reason.clone();
            let _ = tokio::task::spawn_blocking(move || {
                wiki.append_log_with_limit(
                    &format!("## [{date}] regenerate-source | {filename_c} — failed: {reason_c}"),
                    log_max_lines,
                )
            })
            .await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": reason})),
            )
                .into_response();
        }
    };

    // ── Step 3: Staged build + atomic swap + manifest update (blocking, holds mutex) ──
    // Old pages are deleted only AFTER the staged build is committed (H3 fix: no pre-delete).
    let filename_c = filename.clone();
    let manifest_sources_c = prep.manifest_sources;
    let manifest_pages_c = prep.manifest_pages;
    let source_pages_c = prep.source_pages.clone();
    let llm_pages_with_source_c = llm_pages_with_source.clone();
    let source_content_c = prep.source_content.clone();
    let model_c = model.clone();
    let schema_text_c = schema_text.clone();
    let write_result = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || {
            let rebuild_dir = wiki.begin_staged_build()?;
            for (p, _src) in &llm_pages_with_source_c {
                if let Err(e) =
                    wiki.write_staged_page(&rebuild_dir, &p.page_type, &p.slug, &p.content)
                {
                    tracing::warn!("skipping page '{}': {e}", p.slug);
                }
            }
            let committed = match wiki.commit_staged_build(&rebuild_dir) {
                Ok(c) => c,
                Err(e) => {
                    wiki.abort_staged_build(&rebuild_dir);
                    return Err(e);
                }
            };

            // Delete old pages derived from this source only AFTER successful commit (H3 fix)
            if let Err(e) = wiki.delete_page_files(&source_pages_c) {
                tracing::warn!(
                    "failed to delete old pages after per-source regenerate commit: {e}"
                );
            }

            let run_id = WikiManager::new_run_id();
            let prompt_hash =
                WikiManager::hash_content(&wiki.read_ingest_prompt().unwrap_or_default());
            // Use the schema already read before the compiler call (M4 fix: no extra disk read).
            let schema_hash = WikiManager::hash_content(&schema_text_c);

            let mut written_pages = Vec::new();
            let mut new_page_records: Vec<PageRecord> = Vec::new();
            for (ptype, slug) in &committed {
                if let Some((lp, _src)) = llm_pages_with_source_c
                    .iter()
                    .find(|(p, _)| &p.slug == slug)
                    && let Ok(page) = wiki.write_page(ptype, slug, &lp.content)
                {
                    written_pages.push(page);
                    new_page_records.push(PageRecord {
                        slug: slug.clone(),
                        page_type: ptype.clone(),
                        path: format!("pages/{ptype}/{slug}.md"),
                        sources: vec![filename_c.clone()],
                        last_run_id: run_id.clone(),
                        managed_by: "source_ingest".to_string(),
                    });
                }
            }

            let other_page_records: Vec<PageRecord> = manifest_pages_c
                .into_iter()
                .filter(|p| !p.sources.contains(&filename_c) || p.managed_by != "source_ingest")
                .collect();
            let all_page_records: Vec<PageRecord> = new_page_records
                .into_iter()
                .chain(other_page_records)
                .collect();

            let new_source_records: Vec<SourceRecord> = manifest_sources_c
                .into_iter()
                .map(|s| {
                    if s.filename == filename_c {
                        SourceRecord {
                            filename: s.filename,
                            hash: WikiManager::hash_content(&source_content_c),
                            active: s.active,
                            last_run_id: Some(run_id.clone()),
                            source_type: s.source_type,
                        }
                    } else {
                        s
                    }
                })
                .collect();

            if let Err(e) = wiki.write_manifest(&new_source_records, &all_page_records) {
                tracing::warn!("manifest write failed after per-source regenerate: {e}");
            }
            let _ = wiki.append_run(&RunRecord {
                run_id,
                timestamp: chrono::Utc::now().to_rfc3339(),
                model: model_c,
                prompt_hash,
                schema_hash,
                sources: vec![filename_c.clone()],
                status: "success".to_string(),
                pages_written: written_pages.iter().map(|p| p.slug.clone()).collect(),
            });
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Err(e) = wiki.update_index() {
                tracing::warn!("wiki index update failed after per-source regenerate: {e}");
            }
            let _ = wiki.append_log_with_limit(
                &format!(
                    "## [{date}] regenerate-source | {filename_c} → {} page(s)",
                    written_pages.len()
                ),
                log_max_lines,
            );
            Ok::<_, crate::error::ZeniiError>(written_pages)
        })
        .await
    };

    let written_pages = match write_result {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("task panic: {e}")})),
            )
                .into_response();
        }
    };

    if let Err(e) = state
        .wiki
        .lock()
        .await
        .sync_to_memory(state.memory.as_ref())
        .await
    {
        tracing::warn!("wiki memory sync failed: {e}");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(RegenerateResponse {
            sources_processed: 1,
            pages_generated: written_pages.len(),
            message: format!(
                "Regenerated from '{}'; {} pages written.",
                filename,
                written_pages.len()
            ),
        })),
    )
        .into_response()
}

/// POST /wiki/upload — upload a binary or text file for wiki ingestion.
///
/// Accepts multipart/form-data with fields:
/// - `file` (required): raw file bytes + filename
/// - `model` (optional): AI model override
///
/// Binary formats (PDF, DOCX, PPTX, XLSX, images, etc.) are converted to markdown
/// using the configured `DocumentConverter` (default: markitdown subprocess).
/// Text and markdown files are read directly as UTF-8.
///
/// After conversion, the content is fed into the standard wiki ingest pipeline.
pub async fn upload_wiki_source(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    use crate::wiki::convert::convert_file;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut model: Option<String> = None;

    // Extract multipart fields
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Some(name) = field.file_name() {
                    filename = Some(name.to_string());
                }
                match field.bytes().await {
                    Ok(bytes) => file_bytes = Some(bytes.to_vec()),
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": format!("failed to read file field: {e}")})),
                        )
                            .into_response();
                    }
                }
            }
            Some("model") => {
                if let Ok(val) = field.text().await
                    && !val.is_empty()
                {
                    model = Some(val);
                }
            }
            _ => {}
        }
    }

    let bytes = match file_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing 'file' field in multipart form"})),
            )
                .into_response();
        }
    };

    // ── Size guard: reject uploads that exceed the configured source size limit ──
    {
        let max_bytes = state.config.load().wiki_max_source_size_mb * 1024 * 1024;
        if bytes.len() as u64 > max_bytes {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!(
                        "uploaded file too large ({} bytes); limit is {} MiB",
                        bytes.len(),
                        state.config.load().wiki_max_source_size_mb
                    )
                })),
            )
                .into_response();
        }
    }

    let filename = filename.unwrap_or_else(|| "upload.bin".to_string());

    // Write bytes to a temp file for conversion (I2 fix: UUID prefix avoids concurrent-upload races)
    let tmp_name = format!("{}-{}", uuid::Uuid::new_v4(), filename);
    let tmp_path = std::env::temp_dir().join(&tmp_name);
    if let Err(e) = tokio::fs::write(&tmp_path, &bytes).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to write temp file: {e}")})),
        )
            .into_response();
    }

    // Convert to markdown (binary → subprocess, text → UTF-8 read)
    let content = match convert_file(&tmp_path, state.converter.as_ref()).await {
        Ok(text) => text,
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return ZeniiError::Conversion(e.to_string()).into_response();
        }
    };

    // Clean up temp file (best effort)
    let _ = tokio::fs::remove_file(&tmp_path).await;

    // ── Save original bytes and converted markdown separately ────────────────
    // Preserves the immutable original so re-ingestion always converts from the
    // true binary, not a previously-converted markdown copy (H5 fix).
    {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        let filename_c = filename.clone();
        let bytes_c = bytes.clone();
        let content_c = content.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || {
            wiki.save_original_source(&filename_c, &bytes_c)?;
            wiki.save_converted_source(&filename_c, &content_c)?;
            Ok::<_, ZeniiError>(())
        })
        .await
        .unwrap_or_else(|e| Err(ZeniiError::Gateway(e.to_string())))
        {
            tracing::warn!(
                "wiki: failed to persist original/converted source for '{}': {e}",
                filename
            );
            // Non-fatal — continue to ingest even if extra copies could not be written
        }
    }

    // Feed converted markdown into the standard ingest pipeline.
    // ingest_wiki_source also calls save_source() which writes a text copy under
    // sources/ for the LLM to read. We patch the manifest afterward to mark it "binary".
    let ingest_body = IngestRequest {
        content,
        filename: filename.clone(),
        model,
    };

    let response = ingest_wiki_source(State(Arc::clone(&state)), Json(ingest_body))
        .await
        .into_response();

    // ── Patch source_type → "binary" in the manifest ─────────────────────────
    // ingest_wiki_source wrote source_type: "text" (the default); correct it here.
    {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        let filename_c = filename.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || {
            let (mut sources, pages) = wiki.read_manifest_or_bootstrap()?;
            for record in sources.iter_mut() {
                if record.filename == filename_c {
                    record.source_type = "binary".to_string();
                    break;
                }
            }
            wiki.write_manifest(&sources, &pages)?;
            Ok::<_, ZeniiError>(())
        })
        .await
        .unwrap_or_else(|e| Err(ZeniiError::Gateway(e.to_string())))
        {
            tracing::warn!(
                "wiki: failed to patch source_type to 'binary' for '{}': {e}",
                filename
            );
        }
    }

    response
}

/// DELETE /wiki/sources/{filename} — delete a source and cascade-clean its derived pages.
///
/// Pages that only belonged to this source are deleted. Pages shared with other sources
/// are rebuilt from their remaining contributors. User-query pages are never affected.
pub async fn delete_wiki_source(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
    Query(params): Query<DeleteSourceQuery>,
) -> impl IntoResponse {
    use crate::wiki::{PageRecord, RunRecord, WikiManager};

    let log_max_lines = state.config.load().wiki_log_max_lines;

    // ── Step 1: Verify source, read manifest, delete exclusive + shared pages (blocking) ──
    struct DeletePrep {
        manifest_sources: Vec<crate::wiki::SourceRecord>,
        manifest_pages: Vec<PageRecord>,
        deleted_slugs: Vec<String>,
        rebuilt_slugs: Vec<String>,
        remaining_sources: Vec<(String, String)>,
    }
    let prep = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        let filename_c = filename.clone();
        tokio::task::spawn_blocking(move || {
            // Verify exists
            wiki.read_source(&filename_c)
                .map_err(|_| crate::error::ZeniiError::NotFound(filename_c.clone()))?;

            let (manifest_sources, manifest_pages) = wiki.read_manifest_or_bootstrap()?;
            let affected: Vec<&PageRecord> = manifest_pages
                .iter()
                .filter(|r| r.managed_by == "source_ingest" && r.sources.contains(&filename_c))
                .collect();
            let mut exclusive: Vec<PageRecord> = Vec::new();
            let mut shared: Vec<PageRecord> = Vec::new();
            for r in affected {
                if r.sources.len() == 1 {
                    exclusive.push(r.clone());
                } else {
                    shared.push(r.clone());
                }
            }
            let deleted_slugs: Vec<String> = exclusive.iter().map(|r| r.slug.clone()).collect();
            let rebuilt_slugs: Vec<String> = shared.iter().map(|r| r.slug.clone()).collect();

            if let Err(e) = wiki.delete_page_files(&exclusive) {
                tracing::warn!("failed to delete exclusive pages for source '{filename_c}': {e}");
            }
            for r in &shared {
                if let Err(e) = wiki.remove_source_from_page(r, &filename_c) {
                    tracing::warn!("failed to remove source from page '{}': {e}", r.slug);
                }
            }
            let remaining_sources: Vec<(String, String)> = shared
                .iter()
                .flat_map(|r| r.sources.iter().filter(|s| *s != &filename_c))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .filter_map(|f| wiki.read_source(f).ok().map(|c| (f.clone(), c)))
                .collect();

            Ok::<_, crate::error::ZeniiError>(DeletePrep {
                manifest_sources,
                manifest_pages,
                deleted_slugs,
                rebuilt_slugs,
                remaining_sources,
            })
        })
        .await
    };

    let prep = match prep {
        Ok(Ok(p)) => p,
        Ok(Err(crate::error::ZeniiError::NotFound(_))) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("source '{filename}' not found")})),
            )
                .into_response();
        }
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("task panic: {e}")})),
            )
                .into_response();
        }
    };

    // Async: remove deleted pages from memory
    for slug in &prep.deleted_slugs {
        let key = format!("wiki:{slug}");
        if let Err(e) = state.memory.forget(&key).await {
            tracing::warn!("memory forget failed for '{key}': {e}");
        }
    }

    // ── Step 2: Optionally rebuild shared pages via LLM (async) ─────────────
    // run_compiler returns (LlmPage, source_filename) pairs; extract pages for rebuild.
    // Read SCHEMA.md once here (M4 fix: avoid re-reading inside run_compiler for each source).
    let delete_schema_text = if !prep.remaining_sources.is_empty() {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default()
        })
        .await
        .unwrap_or_default()
    } else {
        String::new()
    };
    let rebuilt_pages = if !prep.remaining_sources.is_empty() {
        match run_compiler(
            &state,
            &prep.remaining_sources,
            params.model.as_deref(),
            &delete_schema_text,
        )
        .await
        {
            Ok(llm_pages_with_source) if !llm_pages_with_source.is_empty() => {
                let rebuilt_slugs_c = prep.rebuilt_slugs.clone();
                // Discard source tag — delete_wiki_source manages provenance separately
                let llm_pages_c: Vec<LlmPage> =
                    llm_pages_with_source.into_iter().map(|(p, _)| p).collect();
                let wiki = Arc::clone(&state.wiki).lock_owned().await;
                tokio::task::spawn_blocking(move || {
                    let rebuild_dir = match wiki.begin_staged_build() {
                        Ok(d) => d,
                        Err(_) => return Vec::new(),
                    };
                    for p in &llm_pages_c {
                        if rebuilt_slugs_c.contains(&p.slug) {
                            let _ = wiki.write_staged_page(
                                &rebuild_dir,
                                &p.page_type,
                                &p.slug,
                                &p.content,
                            );
                        }
                    }
                    if wiki.commit_staged_build(&rebuild_dir).is_ok() {
                        let mut pages = Vec::new();
                        for p in &llm_pages_c {
                            if rebuilt_slugs_c.contains(&p.slug)
                                && let Ok(page) = wiki.write_page(&p.page_type, &p.slug, &p.content)
                            {
                                pages.push(page);
                            }
                        }
                        pages
                    } else {
                        wiki.abort_staged_build(&rebuild_dir);
                        Vec::new()
                    }
                })
                .await
                .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    // ── Step 3: Update manifest + index + log (blocking, holds mutex) ────────
    let filename_c = filename.clone();
    let manifest_sources_c = prep.manifest_sources;
    let manifest_pages_c = prep.manifest_pages;
    let deleted_slugs_c = prep.deleted_slugs.clone();
    let rebuilt_slugs_c = prep.rebuilt_slugs.clone();
    let rebuilt_pages_c = rebuilt_pages.clone();
    let model_c = params.model.clone();
    {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || {
            let run_id = WikiManager::new_run_id();
            let new_sources: Vec<_> = manifest_sources_c
                .into_iter()
                .filter(|s| s.filename != filename_c)
                .collect();
            let new_pages: Vec<PageRecord> = manifest_pages_c
                .into_iter()
                .filter(|r| !deleted_slugs_c.contains(&r.slug))
                .map(|mut r| {
                    if rebuilt_slugs_c.contains(&r.slug) {
                        r.sources.retain(|s| s != &filename_c);
                        r.last_run_id = run_id.clone();
                    }
                    r
                })
                .collect();
            if let Err(e) = wiki.write_manifest(&new_sources, &new_pages) {
                tracing::warn!("manifest write failed after delete-source: {e}");
            }
            // Delete source file after manifest is safely updated (I1 fix: safe ordering)
            if let Err(e) = wiki.delete_source_file(&filename_c) {
                tracing::warn!("failed to delete source file '{}': {e}", filename_c);
            }
            let _ = wiki.append_run(&RunRecord {
                run_id,
                timestamp: chrono::Utc::now().to_rfc3339(),
                model: model_c,
                prompt_hash: WikiManager::hash_content(
                    &wiki.read_ingest_prompt().unwrap_or_default(),
                ),
                schema_hash: WikiManager::hash_content(
                    &std::fs::read_to_string(wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default(),
                ),
                sources: vec![filename_c.clone()],
                status: "success".to_string(),
                pages_written: rebuilt_pages_c.iter().map(|p| p.slug.clone()).collect(),
            });
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Err(e) = wiki.update_index() {
                tracing::warn!("wiki index update failed after delete-source: {e}");
            }
            if let Err(e) = wiki.append_log_with_limit(
                &format!(
                    "## [{date}] delete-source | {filename_c} — {} deleted, {} rebuilt",
                    deleted_slugs_c.len(),
                    rebuilt_pages_c.len()
                ),
                log_max_lines,
            ) {
                tracing::warn!("wiki log append failed: {e}");
            }
        })
        .await
        .ok();
    }

    if let Err(e) = state
        .wiki
        .lock()
        .await
        .sync_to_memory(state.memory.as_ref())
        .await
    {
        tracing::warn!("wiki memory sync failed: {e}");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(DeleteSourceResponse {
            filename,
            deleted_pages: prep.deleted_slugs,
            rebuilt_pages: rebuilt_pages.iter().map(|p| p.slug.clone()).collect(),
            message: "Source deleted and wiki updated.".to_string(),
        })),
    )
        .into_response()
}

/// Shared compiler pipeline: call the LLM to generate wiki pages from a list of sources.
///
/// Accepts `schema` as a pre-read string to avoid redundant disk I/O when called in a
/// loop over multiple sources (M4 fix: SCHEMA.md read once by caller, not per-call).
/// Returns `Ok(pages_with_source)` where each entry is `(LlmPage, source_filename)` so
/// callers can track which source produced each page (C6 fix: correct provenance).
/// Returns `Err(())` when the agent or JSON parsing fails.
async fn run_compiler(
    state: &AppState,
    sources: &[(String, String)],
    model: Option<&str>,
    schema: &str,
) -> Result<Vec<(LlmPage, String)>, String> {
    use crate::ai::resolve_agent;

    if sources.is_empty() {
        return Ok(Vec::new());
    }

    // Read INGEST_PROMPT.md (user-editable) at runtime (blocking, holds mutex).
    // SCHEMA.md is passed in by the caller to avoid N disk reads for N sources (M4 fix).
    let ingest_prompt = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        tokio::task::spawn_blocking(move || wiki.read_ingest_prompt().unwrap_or_default())
            .await
            .unwrap_or_default()
    };

    let system_prompt = format!("{ingest_prompt}\n\n{schema}");

    let mut all_pages: Vec<(LlmPage, String)> = Vec::new();

    for (filename, content) in sources {
        // Provide existing wiki context so LLM can update/merge pages (blocking, holds mutex).
        let (current_index, summaries) = {
            let wiki = Arc::clone(&state.wiki).lock_owned().await;
            tokio::task::spawn_blocking(move || {
                let index = wiki.read_index().unwrap_or_default();
                let summaries: Vec<String> = wiki
                    .list_pages()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        let tldr = if p.tldr.trim().is_empty() {
                            "(no summary)".to_string()
                        } else {
                            p.tldr.lines().next().unwrap_or("").to_string()
                        };
                        format!("{}: {tldr}", p.slug)
                    })
                    .collect();
                (index, summaries.join("\n"))
            })
            .await
            .unwrap_or_default()
        };
        let user_prompt = if all_pages.is_empty()
            && (current_index.is_empty() || current_index.contains("No pages yet"))
        {
            format!("Filename: {filename}\n\nContent:\n{content}")
        } else {
            format!(
                "Filename: {filename}\n\nCurrent wiki index:\n{current_index}\n\nPage summaries:\n{summaries}\n\nNew source content:\n{content}"
            )
        };

        let agent = resolve_agent(model, state, None, Some(&system_prompt), "wiki")
            .await
            .map_err(|e| e.to_string())?;

        let response = agent
            .prompt(&user_prompt)
            .await
            .map_err(|e| e.to_string())?;

        // Strip optional markdown code fences
        let raw = response
            .output
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        match serde_json::from_str::<Vec<LlmPage>>(raw) {
            Ok(pages) => {
                // Tag each page with the source filename that produced it
                all_pages.extend(pages.into_iter().map(|p| (p, filename.clone())));
            }
            Err(e) => {
                tracing::warn!(
                    "LLM JSON parse failed: {e}\nRaw response (first 500 chars): {}",
                    &raw[..500.min(raw.len())]
                );
            }
        }
    }

    if all_pages.is_empty() {
        Err("LLM returned no parseable pages.".to_string())
    } else {
        Ok(all_pages)
    }
}

/// POST /wiki/sync — sync compiled wiki pages into the memory store.
pub async fn sync_wiki_to_memory(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state
        .wiki
        .lock()
        .await
        .sync_to_memory(state.memory.as_ref())
        .await
    {
        Ok(count) => (StatusCode::OK, Json(serde_json::json!({"synced": count}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /wiki/graph — return wiki knowledge graph (nodes + edges).
pub async fn get_wiki_graph(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let result = tokio::task::spawn_blocking(move || wiki.graph()).await;
    match result {
        Ok(Ok(graph)) => (StatusCode::OK, Json(serde_json::json!(graph))).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// POST /wiki/query — answer a question using the wiki as a knowledge base.
///
/// Reads the current index and all page bodies as context, calls the configured LLM to
/// synthesize an answer with citations. Optionally saves the answer as a query page.
pub async fn query_wiki(
    State(state): State<Arc<AppState>>,
    Json(body): Json<QueryRequest>,
) -> impl IntoResponse {
    use crate::ai::resolve_agent;

    let log_max_lines = state.config.load().wiki_log_max_lines;

    // Read the current index, pages, and schema (blocking, holds mutex).
    let (index, pages, schema) = {
        let wiki = Arc::clone(&state.wiki).lock_owned().await;
        match tokio::task::spawn_blocking(move || {
            let index = wiki.read_index()?;
            let pages = wiki.list_pages()?;
            let schema =
                std::fs::read_to_string(wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default();
            Ok::<_, crate::error::ZeniiError>((index, pages, schema))
        })
        .await
        {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("task panic: {e}")})),
                )
                    .into_response();
            }
        }
    };

    // Select only relevant pages to avoid sending the full wiki for large corpora.
    let selected = score_pages_for_query(&pages, &body.question);
    let pages_context: String = selected
        .iter()
        .map(|p| format!("### {}\n{}", p.slug, p.body))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let system_prompt = format!(
        r#"You are a wiki query assistant. Answer questions using ONLY the wiki pages provided. Cite sources using the page slug (e.g. "per [[page-slug]]").

{schema}

## Current Wiki Index
{index}

## Wiki Pages
{pages_context}"#
    );

    let agent = match resolve_agent(
        body.model.as_deref(),
        &state,
        None,
        Some(&system_prompt),
        "wiki",
    )
    .await
    {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("no agent configured: {e}")})),
            )
                .into_response();
        }
    };

    let response = match agent.prompt(&body.question).await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let answer = response.output;
    let citations = extract_wikilink_citations(&answer);

    // Optionally save the answer as a query page + update index + log (blocking, holds mutex).
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let answer_c = answer.clone();
    let question_c = body.question.clone();
    let save_c = body.save.unwrap_or(false);
    let saved_page = tokio::task::spawn_blocking(move || {
        use crate::wiki::{PageRecord, RunRecord, WikiManager};
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let log_entry = format!("## [{date}] query | {question_c}");
        if let Err(e) = wiki.append_log_with_limit(&log_entry, log_max_lines) {
            tracing::warn!("wiki log append failed after query: {e}");
        }
        if save_c {
            // L11: append 8-char SHA-256 hex prefix to ensure slug uniqueness across similar questions
            let base_slug = slugify_question(&question_c);
            let hash_prefix = {
                use sha2::{Digest, Sha256};
                let h = Sha256::digest(question_c.as_bytes());
                format!("{:02x}{:02x}{:02x}{:02x}", h[0], h[1], h[2], h[3])
            };
            let slug = format!("{base_slug}-{hash_prefix}");
            // Truncate to 64 chars (slug validation limit)
            let slug: String = slug.chars().take(64).collect();
            let first_line = answer_c.lines().next().unwrap_or("").to_string();
            let content = format!(
                "---\ntitle: \"{question_c}\"\ntype: query\ntags: []\nsources: []\nupdated: {date}\n---\n\n## TLDR\n{first_line}\n\n## Body\n{answer_c}\n",
            );
            if let Ok(page) = wiki.write_page("queries", &slug, &content) {
                // L12: record PageRecord + RunRecord for saved query pages
                if let Ok((sources, mut page_records)) = wiki.read_manifest_or_bootstrap() {
                    let run_id = WikiManager::new_run_id();
                    page_records.retain(|r| r.slug != slug);
                    page_records.push(PageRecord {
                        slug: slug.clone(),
                        page_type: "query".to_string(),
                        path: format!("pages/queries/{slug}.md"),
                        sources: vec![],
                        last_run_id: run_id.clone(),
                        managed_by: "user_query".to_string(),
                    });
                    if let Err(e) = wiki.write_manifest(&sources, &page_records) {
                        tracing::warn!("manifest write failed after saving query page: {e}");
                    }
                    let _ = wiki.append_run(&RunRecord {
                        run_id,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        model: None,
                        prompt_hash: String::new(),
                        schema_hash: String::new(),
                        sources: vec![],
                        status: "success".to_string(),
                        pages_written: vec![slug.clone()],
                    });
                }
                if let Err(e) = wiki.update_index() {
                    tracing::warn!("wiki index update failed after saving query page: {e}");
                }
                return Some(page);
            }
        }
        None
    })
    .await
    .unwrap_or(None);

    (
        StatusCode::OK,
        Json(serde_json::json!(QueryResponse {
            answer,
            citations,
            saved_page,
        })),
    )
        .into_response()
}

/// POST /wiki/lint — run deterministic structural lint over all wiki pages.
///
/// Checks for: broken wikilinks, orphan pages, missing index entries, and pages
/// without an `updated` frontmatter field. Appends a log entry with the summary.
/// Accepts optional JSON body `{ "auto_fix": true }` to auto-fix deterministic issues.
pub async fn lint_wiki(
    State(state): State<Arc<AppState>>,
    body: Option<Json<LintRequest>>,
) -> impl IntoResponse {
    let auto_fix = body.as_ref().and_then(|b| b.auto_fix).unwrap_or(false);
    let log_max_lines = state.config.load().wiki_log_max_lines;

    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let lint_result = tokio::task::spawn_blocking(move || {
        let issues = wiki.lint()?;
        let (issues, fixed) = if auto_fix && !issues.is_empty() {
            match wiki.lint_fix(&issues) {
                Ok((f, r)) => (r, f),
                Err(e) => {
                    tracing::warn!("lint_fix failed: {e}");
                    (issues, Vec::new())
                }
            }
        } else {
            (issues, Vec::new())
        };

        let n = issues.len();
        let summary = if n == 0 {
            "No issues found.".to_string()
        } else {
            let mut counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for issue in &issues {
                *counts.entry(issue.kind.clone()).or_insert(0) += 1;
            }
            let breakdown = counts
                .iter()
                .map(|(k, v)| format!("{v} {k}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{n} issue(s): {breakdown}")
        };

        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let fixed_summary = if fixed.is_empty() {
            String::new()
        } else {
            format!(" | auto-fixed {} issue(s)", fixed.len())
        };
        let log_entry = format!("## [{date}] lint | {summary}{fixed_summary}");
        if let Err(e) = wiki.append_log_with_limit(&log_entry, log_max_lines) {
            tracing::warn!("wiki log append failed after lint: {e}");
        }

        Ok::<_, crate::error::ZeniiError>((issues, fixed, summary))
    })
    .await;

    match lint_result {
        Ok(Ok((issues, fixed, summary))) => (
            StatusCode::OK,
            Json(serde_json::json!(LintResponse {
                issues,
                fixed,
                summary
            })),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// GET /wiki/prompt — read current INGEST_PROMPT.md content.
pub async fn get_wiki_prompt(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let result = tokio::task::spawn_blocking(move || wiki.read_ingest_prompt()).await;
    match result {
        Ok(Ok(content)) => (StatusCode::OK, Json(PromptResponse { content })).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// PUT /wiki/prompt — write new content to INGEST_PROMPT.md.
pub async fn set_wiki_prompt(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetPromptRequest>,
) -> impl IntoResponse {
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let content = body.content.clone();
    let result = tokio::task::spawn_blocking(move || wiki.set_prompt(&content)).await;
    match result {
        Ok(Ok(())) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Ok(Err(ZeniiError::Validation(msg))) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error_code": "ZENII_VALIDATION",
                "message": msg
            })),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

/// DELETE /wiki/sources — delete all source files, clear manifest source records, and remove ingest pages.
pub async fn delete_all_wiki_sources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let log_max_lines = state.config.load().wiki_log_max_lines;
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let delete_result =
        tokio::task::spawn_blocking(move || {
            let (deleted, ingest_pages) = wiki.delete_all_sources()?;
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Err(e) = wiki.update_index() {
                tracing::warn!("wiki index update failed after delete-all-sources: {e}");
            }
            let _ = wiki.append_log_with_limit(&format!(
            "## [{date}] delete-all-sources | {deleted} source(s) deleted, {} pages removed",
            ingest_pages.len()
        ), log_max_lines);
            Ok::<_, crate::error::ZeniiError>((deleted, ingest_pages))
        })
        .await;

    let (deleted, ingest_pages) = match delete_result {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("task panic: {e}")})),
            )
                .into_response();
        }
    };

    // Remove deleted pages from the memory store (async)
    for page in &ingest_pages {
        let key = format!("wiki:{}", page.slug);
        if let Err(e) = state.memory.forget(&key).await {
            tracing::warn!(
                "failed to forget wiki page '{}' from memory: {e}",
                page.slug
            );
        }
    }

    (
        StatusCode::OK,
        Json(DeleteSourcesResponse {
            message: format!(
                "Deleted {deleted} source files and {} pages",
                ingest_pages.len()
            ),
            deleted,
        }),
    )
        .into_response()
}

/// DELETE /wiki/pages — delete all wiki pages and reset index.md.
pub async fn delete_wiki_pages(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let wiki = Arc::clone(&state.wiki).lock_owned().await;
    let result = tokio::task::spawn_blocking(move || wiki.delete_all_pages()).await;
    match result {
        Ok(Ok(deleted)) => {
            // Sync memory to remove stale wiki:* keys (I7 fix)
            if let Err(e) = state
                .wiki
                .lock()
                .await
                .sync_to_memory(state.memory.as_ref())
                .await
            {
                tracing::warn!("wiki memory sync failed after delete-pages: {e}");
            }
            (
                StatusCode::OK,
                Json(DeletePagesResponse {
                    message: format!("Deleted {deleted} wiki pages"),
                    deleted,
                }),
            )
                .into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("task panic: {e}")})),
        )
            .into_response(),
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Select the most relevant pages for a query using keyword scoring.
///
/// Tokenises the question, removes English stop-words, then scores each page by
/// keyword hits: title/TLDR matches weight 3x, body matches weight 1x.
/// Returns up to `QUERY_MAX_PAGES` pages with score >= `QUERY_MIN_SCORE`.
/// Falls back to returning all pages when the wiki is small enough to fit within
/// the limit.
fn score_pages_for_query<'a>(pages: &'a [WikiPage], question: &str) -> Vec<&'a WikiPage> {
    const STOP_WORDS: &[&str] = &[
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall", "can",
        "what", "how", "why", "when", "where", "who", "which", "that", "this", "these", "those",
        "in", "on", "at", "to", "for", "of", "with", "by", "from", "and", "or", "not", "it", "its",
        "i", "my", "me", "you", "your", "we", "our", "they", "their",
    ];

    let keywords: Vec<String> = question
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !STOP_WORDS.contains(w))
        .map(|w| w.to_string())
        .collect();

    // If no meaningful keywords or wiki is already small, return all pages up to the limit.
    if keywords.is_empty() || pages.len() <= QUERY_MAX_PAGES {
        return pages.iter().take(QUERY_MAX_PAGES).collect();
    }

    let mut scored: Vec<(&WikiPage, u32)> = pages
        .iter()
        .map(|page| {
            let title_tldr = format!("{} {}", page.title.to_lowercase(), page.tldr.to_lowercase());
            let body = page.body.to_lowercase();
            let score = keywords.iter().fold(0u32, |acc, kw| {
                let title_hits = title_tldr.matches(kw.as_str()).count() as u32;
                let body_hits = body.matches(kw.as_str()).count() as u32;
                acc + title_hits * 3 + body_hits
            });
            (page, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored
        .into_iter()
        .filter(|(_, score)| *score >= QUERY_MIN_SCORE)
        .take(QUERY_MAX_PAGES)
        .map(|(page, _)| page)
        .collect()
}

/// Extract [[slug]] references from any text.
fn extract_wikilink_citations(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("[[") {
        rest = &rest[open + 2..];
        if let Some(close) = rest.find("]]") {
            let link = rest[..close].to_string();
            if !link.is_empty() && !links.contains(&link) {
                links.push(link);
            }
            rest = &rest[close + 2..];
        } else {
            break;
        }
    }
    links
}

/// Convert a question string into a valid kebab-case page slug (max 64 chars).
fn slugify_question(question: &str) -> String {
    question
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(64)
        .collect()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{delete, get, post};
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/wiki", get(list_wiki_pages))
            .route("/wiki/search", get(search_wiki_pages))
            .route("/wiki/ingest", post(ingest_wiki_source))
            .route("/wiki/sync", post(sync_wiki_to_memory))
            .route("/wiki/graph", get(get_wiki_graph))
            .route("/wiki/query", post(query_wiki))
            .route("/wiki/lint", post(lint_wiki))
            .route(
                "/wiki/sources",
                get(list_wiki_sources).delete(delete_all_wiki_sources),
            )
            .route("/wiki/sources/{filename}", delete(delete_wiki_source))
            .route("/wiki/dir", get(get_wiki_dir))
            .route("/wiki/prompt", get(get_wiki_prompt).put(set_wiki_prompt))
            .route("/wiki/pages", delete(delete_wiki_pages))
            .route("/wiki/regenerate", post(regenerate_wiki))
            .route("/wiki/{slug}", get(get_wiki_page))
            .with_state(state)
    }

    /// H1: GET /wiki with no pages → 200 with paginated response containing empty pages array
    #[tokio::test]
    async fn wiki_list_empty_returns_200_empty_array() {
        let (_dir, state) = test_state().await;

        let req = Request::builder().uri("/wiki").body(Body::empty()).unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let resp_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let pages = resp_json["pages"].as_array().expect("expected pages array");
        assert!(pages.is_empty(), "expected empty pages array");
    }

    /// H2: GET /wiki → 200 with paginated response object containing pages array
    #[tokio::test]
    async fn wiki_list_returns_pages() {
        let (_dir, state) = test_state().await;

        let req = Request::builder().uri("/wiki").body(Body::empty()).unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        // Body must be a valid paginated response object with pages, total, limit, offset fields
        let resp_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            resp_json["pages"].is_array(),
            "expected pages array in response"
        );
        assert!(
            resp_json["total"].is_number(),
            "expected total count in response"
        );
    }

    /// H3: GET /wiki/{slug} for a page that exists → 200
    #[tokio::test]
    async fn wiki_get_existing_returns_200() {
        let (_dir, state) = test_state().await;

        // Seed a page via ingest so the slug exists
        let ingest_body = serde_json::json!({
            "filename": "test-page.md",
            "content": "---\ntitle: \"Test Page\"\ntype: concept\ntags: []\nsources: []\nupdated: 2026-01-01\n---\n\n## TLDR\nTest page for handler tests.\n\n## Body\nContent here.\n"
        });
        let seed_req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&ingest_body).unwrap()))
            .unwrap();
        let seed_resp = app(state.clone()).oneshot(seed_req).await.unwrap();
        assert_eq!(
            seed_resp.status(),
            StatusCode::OK,
            "ingest seed must succeed"
        );

        let req = Request::builder()
            .uri("/wiki/test-page")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// H4: GET /wiki/{slug} for a page that does not exist → 404
    #[tokio::test]
    async fn wiki_get_missing_returns_404() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/no-such-page")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// H5: GET /wiki/search?q=test → 200
    #[tokio::test]
    async fn wiki_search_returns_200() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/search?q=test")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// H6: POST /wiki/ingest with a valid JSON body → 200
    #[tokio::test]
    async fn wiki_ingest_returns_200() {
        let (_dir, state) = test_state().await;

        let body = serde_json::to_string(&serde_json::json!({
            "content": "# Hello\nThis is a test document.",
            "filename": "hello.md"
        }))
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// H7: POST /wiki/sync → 200 and body contains a `synced` key
    #[tokio::test]
    async fn wiki_sync_returns_200_with_synced_count() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/sync")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            val.get("synced").is_some(),
            "response body must contain 'synced' key"
        );
    }

    /// H8: GET /wiki/graph → 200 and body contains `nodes` and `edges` keys
    #[tokio::test]
    async fn wiki_graph_returns_200_with_nodes_edges() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/graph")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            val.get("nodes").is_some(),
            "response body must contain 'nodes' key"
        );
        assert!(
            val.get("edges").is_some(),
            "response body must contain 'edges' key"
        );
    }

    /// H9: POST /wiki/ingest falls back to a single topic page when no LLM is configured;
    /// response body must have a `pages` array and a `message` string.
    #[tokio::test]
    async fn wiki_ingest_fallback_returns_pages_array() {
        let (_dir, state) = test_state().await;

        let body = serde_json::to_string(&serde_json::json!({
            "content": "# Hello\nThis is a test document.",
            "filename": "hello.md"
        }))
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(val.get("pages").is_some(), "response must have 'pages' key");
        assert!(
            val.get("message").is_some(),
            "response must have 'message' key"
        );
        let pages = val["pages"].as_array().unwrap();
        assert!(!pages.is_empty(), "at least one page must be returned");
    }

    /// H10: POST /wiki/ingest response must include a non-empty `primary_slug` field.
    #[tokio::test]
    async fn wiki_ingest_response_has_primary_slug() {
        let (_dir, state) = test_state().await;

        let body = serde_json::to_string(&serde_json::json!({
            "content": "# My Doc\nSome content here.",
            "filename": "my-doc.md"
        }))
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            val.get("primary_slug").is_some(),
            "response must have 'primary_slug'"
        );
        let slug = val["primary_slug"].as_str().unwrap_or("");
        assert!(!slug.is_empty(), "primary_slug must not be empty");
    }

    /// H11: POST /wiki/lint → 200 with `issues` array and `summary` string.
    #[tokio::test]
    async fn wiki_lint_returns_200_with_issues_and_summary() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/lint")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(val.get("issues").is_some(), "response must have 'issues'");
        assert!(val.get("summary").is_some(), "response must have 'summary'");
        assert!(val["issues"].is_array(), "'issues' must be an array");
    }

    /// H12: POST /wiki/query without a configured LLM must return valid JSON (not panic).
    #[tokio::test]
    async fn wiki_query_without_llm_returns_valid_json() {
        let (_dir, state) = test_state().await;

        let body = serde_json::to_string(&serde_json::json!({
            "question": "What is attention?"
        }))
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/query")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        // Either 500 (no agent) or 200 — both acceptable; response must be valid JSON.
        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let _val: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response must be valid JSON");
    }

    /// H13: Ingesting a second document that overlaps with existing pages must not duplicate slugs.
    /// The same slug written twice should exist exactly once in the page list.
    #[tokio::test]
    async fn wiki_second_ingest_does_not_duplicate_slug() {
        let (_dir, state) = test_state().await;

        // First ingest — creates "my-doc" in topics/
        let body1 = serde_json::to_string(&serde_json::json!({
            "content": "---\ntitle: \"My Doc\"\ntype: topic\ntags: []\nsources: []\nupdated: 2026-01-01\n---\n\n## TLDR\nFirst ingest.\n\n## Body\nOriginal content.\n",
            "filename": "my-doc.md"
        }))
        .unwrap();
        let req1 = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(body1))
            .unwrap();
        let resp1 = app(state.clone()).oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK, "first ingest must succeed");
        let bytes1 = axum::body::to_bytes(resp1.into_body(), 16 * 1024)
            .await
            .unwrap();
        let val1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
        let slug = val1["primary_slug"]
            .as_str()
            .unwrap_or("my-doc")
            .to_string();

        // Second ingest — same slug, updated content (LLM unavailable, falls back to single page)
        let body2 = serde_json::to_string(&serde_json::json!({
            "content": "---\ntitle: \"My Doc\"\ntype: topic\ntags: []\nsources: []\nupdated: 2026-04-01\n---\n\n## TLDR\nUpdated content.\n\n## Body\nRevised content referencing [[my-doc]].\n",
            "filename": "my-doc.md"
        }))
        .unwrap();
        let req2 = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(body2))
            .unwrap();
        let resp2 = app(state.clone()).oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK, "second ingest must succeed");

        // List all pages and count occurrences of the slug
        let list_req = Request::builder().uri("/wiki").body(Body::empty()).unwrap();
        let list_resp = app(state).oneshot(list_req).await.unwrap();
        let list_bytes = axum::body::to_bytes(list_resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let resp_json: serde_json::Value = serde_json::from_slice(&list_bytes).unwrap();
        let pages = resp_json["pages"].as_array().expect("expected pages array");
        let count = pages
            .iter()
            .filter(|p| p["slug"].as_str() == Some(&slug))
            .count();
        assert_eq!(
            count, 1,
            "slug '{slug}' must appear exactly once after two ingests"
        );
    }

    /// H14: When index.md is read-only, update_index() fails and ingest must return
    /// non-200 with a `pages_written` field in the error body (unix-only).
    #[cfg(unix)]
    #[tokio::test]
    async fn wiki_ingest_returns_500_when_index_update_fails() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, state) = test_state().await;

        // Make index.md read-only so update_index() cannot overwrite it.
        let index_path = state.wiki.lock().await.wiki_dir().join("index.md");
        std::fs::set_permissions(&index_path, std::fs::Permissions::from_mode(0o444)).unwrap();

        let body = serde_json::to_string(&serde_json::json!({
            "content": "# Hello\nThis is a test document.",
            "filename": "hello.md"
        }))
        .unwrap();
        let req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app(state.clone()).oneshot(req).await.unwrap();

        // Restore permissions before asserting so cleanup works regardless.
        std::fs::set_permissions(&index_path, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "ingest must return 500 when index.md is read-only"
        );

        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            val.get("pages_written").is_some(),
            "error body must include 'pages_written' so caller knows what was created"
        );
    }

    /// H15: GET /wiki/sources with no sources → 200 and a JSON array.
    #[tokio::test]
    async fn wiki_list_sources_empty_returns_200() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/sources")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(val.is_array(), "response must be a JSON array");
    }

    /// H16: After ingesting a source, GET /wiki/sources must contain the filename.
    #[tokio::test]
    async fn wiki_list_sources_after_ingest_contains_filename() {
        let (_dir, state) = test_state().await;

        let ingest_body = serde_json::json!({
            "filename": "my-source.md",
            "content": "# Hello\nSome content."
        });
        let req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&ingest_body).unwrap()))
            .unwrap();
        let resp = app(state.clone()).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "ingest must succeed");

        let list_req = Request::builder()
            .uri("/wiki/sources")
            .body(Body::empty())
            .unwrap();
        let list_resp = app(state).oneshot(list_req).await.unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(list_resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let sources: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let found = sources
            .iter()
            .any(|s| s["filename"].as_str() == Some("my-source.md"));
        assert!(found, "sources list must contain 'my-source.md'");
    }

    /// H17: GET /wiki/dir → 200 with a `path` key that is a non-empty string.
    #[tokio::test]
    async fn wiki_dir_returns_200_with_path() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/dir")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let path = val["path"].as_str().unwrap_or("");
        assert!(!path.is_empty(), "path must be a non-empty string");
    }

    /// H18: GET /wiki/prompt → 200 with a `content` key.
    #[tokio::test]
    async fn wiki_get_prompt_returns_200_with_content() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/prompt")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            val.get("content").is_some(),
            "response must contain 'content' key"
        );
    }

    /// H19: PUT /wiki/prompt with valid content → 200.
    #[tokio::test]
    async fn wiki_set_prompt_returns_200() {
        let (_dir, state) = test_state().await;

        let body = serde_json::json!({"content": "You are a wiki LLM. Extract key concepts."});
        let req = Request::builder()
            .method("PUT")
            .uri("/wiki/prompt")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// H20: PUT /wiki/prompt with empty content → 400.
    #[tokio::test]
    async fn wiki_set_prompt_empty_returns_400() {
        let (_dir, state) = test_state().await;

        let body = serde_json::json!({"content": ""});
        let req = Request::builder()
            .method("PUT")
            .uri("/wiki/prompt")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// H21: PUT then GET /wiki/prompt → GET returns the content that was PUT.
    #[tokio::test]
    async fn wiki_set_prompt_roundtrip() {
        let (_dir, state) = test_state().await;

        let new_prompt = "Custom wiki extraction instructions for roundtrip test.";
        let put_body = serde_json::json!({"content": new_prompt});
        let put_req = Request::builder()
            .method("PUT")
            .uri("/wiki/prompt")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&put_body).unwrap()))
            .unwrap();
        let put_resp = app(state.clone()).oneshot(put_req).await.unwrap();
        assert_eq!(put_resp.status(), StatusCode::OK, "PUT must succeed");

        let get_req = Request::builder()
            .uri("/wiki/prompt")
            .body(Body::empty())
            .unwrap();
        let get_resp = app(state).oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(get_resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let returned = val["content"].as_str().unwrap_or("");
        assert!(
            returned.contains(new_prompt),
            "GET must return the content set by PUT, got: {returned}"
        );
    }

    /// H22: DELETE /wiki/sources → 200 with a `deleted` key.
    #[tokio::test]
    async fn wiki_delete_all_sources_returns_200() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("DELETE")
            .uri("/wiki/sources")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            val.get("deleted").is_some(),
            "response must contain 'deleted' key"
        );
    }

    /// H23: DELETE /wiki/pages → 200 with a `deleted` key.
    #[tokio::test]
    async fn wiki_delete_all_pages_returns_200() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("DELETE")
            .uri("/wiki/pages")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            val.get("deleted").is_some(),
            "response must contain 'deleted' key"
        );
    }

    /// H24: POST ingest then DELETE /wiki/sources/{filename} → 200.
    #[tokio::test]
    async fn wiki_delete_source_after_ingest_returns_200() {
        let (_dir, state) = test_state().await;

        // Ingest a source so it exists
        let ingest_body = serde_json::json!({
            "filename": "deleteme.md",
            "content": "# Delete Me\nThis source will be deleted."
        });
        let ingest_req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&ingest_body).unwrap()))
            .unwrap();
        let ingest_resp = app(state.clone()).oneshot(ingest_req).await.unwrap();
        assert_eq!(ingest_resp.status(), StatusCode::OK, "ingest must succeed");

        // Now delete the source
        let del_req = Request::builder()
            .method("DELETE")
            .uri("/wiki/sources/deleteme.md")
            .body(Body::empty())
            .unwrap();
        let del_resp = app(state).oneshot(del_req).await.unwrap();
        assert_eq!(del_resp.status(), StatusCode::OK);
    }

    /// H25: DELETE /wiki/sources/{filename} for a non-existent file → 404.
    #[tokio::test]
    async fn wiki_delete_source_missing_returns_404() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("DELETE")
            .uri("/wiki/sources/ghost.md")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// H26: POST /wiki/regenerate with empty wiki → 200 with `sources_processed` = 0.
    #[tokio::test]
    async fn wiki_regenerate_empty_returns_200() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/regenerate")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            val["sources_processed"].as_u64(),
            Some(0),
            "empty wiki must report sources_processed = 0"
        );
    }

    /// H27: After ingesting a source, GET /wiki/search?q=<keyword> returns a non-empty array.
    #[tokio::test]
    async fn wiki_search_after_ingest_finds_page() {
        let (_dir, state) = test_state().await;

        // Ingest a page containing the keyword "foxglove"
        let ingest_body = serde_json::json!({
            "filename": "foxglove.md",
            "content": "# Foxglove\nFoxglove is a plant with bell-shaped flowers used in medicine."
        });
        let ingest_req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&ingest_body).unwrap()))
            .unwrap();
        let ingest_resp = app(state.clone()).oneshot(ingest_req).await.unwrap();
        assert_eq!(ingest_resp.status(), StatusCode::OK, "ingest must succeed");

        // Search for the keyword
        let search_req = Request::builder()
            .uri("/wiki/search?q=foxglove")
            .body(Body::empty())
            .unwrap();
        let search_resp = app(state).oneshot(search_req).await.unwrap();
        assert_eq!(search_resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(search_resp.into_body(), 16 * 1024)
            .await
            .unwrap();
        let results: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        assert!(
            !results.is_empty(),
            "search for 'foxglove' must return at least one result"
        );
    }
}
