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

/// GET /wiki — list all wiki pages.
pub async fn list_wiki_pages(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.wiki.list_pages() {
        Ok(pages) => (StatusCode::OK, Json(serde_json::json!(pages))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /wiki/{slug} — fetch a single wiki page by slug.
pub async fn get_wiki_page(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.wiki.get_page(&slug) {
        Ok(Some(page)) => (StatusCode::OK, Json(serde_json::json!(page))).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /wiki/search?q= — full-text search over wiki pages.
pub async fn search_wiki_pages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = params.q.as_deref().unwrap_or("");
    match state.wiki.search_pages(q) {
        Ok(pages) => (StatusCode::OK, Json(serde_json::json!(pages))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
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
/// Uses staged builds (write to .rebuild/ first, swap on success) and updates the manifest
/// so future regenerate/delete operations have accurate source lineage.
pub async fn ingest_wiki_source(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IngestRequest>,
) -> impl IntoResponse {
    use crate::wiki::{PageRecord, RunRecord, SourceRecord, WikiManager};

    // Save the raw source for future re-ingestion.
    if let Err(e) = state.wiki.save_source(&body.filename, &body.content) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to save source: {e}")})),
        )
            .into_response();
    }

    let run_id = WikiManager::new_run_id();
    let source_hash = WikiManager::hash_content(&body.content);
    let prompt_hash =
        WikiManager::hash_content(&state.wiki.read_ingest_prompt().unwrap_or_default());
    let schema_hash = WikiManager::hash_content(
        &std::fs::read_to_string(state.wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default(),
    );

    // Attempt LLM-driven multi-page generation via shared compiler pipeline.
    let llm_result = run_compiler(
        &state,
        &[(body.filename.clone(), body.content.clone())],
        body.model.as_deref(),
    )
    .await;

    let (pages, used_llm) = if let Ok(llm_pages) = llm_result {
        // Staged build: write to .rebuild/, then commit
        match state.wiki.begin_staged_build() {
            Ok(rebuild_dir) => {
                let mut staged = Vec::new();
                for p in &llm_pages {
                    if state
                        .wiki
                        .write_staged_page(&rebuild_dir, &p.page_type, &p.slug, &p.content)
                        .is_ok()
                    {
                        staged.push(p.clone());
                    }
                }
                match state.wiki.commit_staged_build(&rebuild_dir) {
                    Ok(_) => {
                        let mut written = Vec::new();
                        for p in staged {
                            if let Ok(page) =
                                state.wiki.write_page(&p.page_type, &p.slug, &p.content)
                            {
                                written.push(page);
                            }
                        }
                        (written, true)
                    }
                    Err(e) => {
                        tracing::warn!("staged build commit failed: {e}");
                        state.wiki.abort_staged_build(&rebuild_dir);
                        (Vec::new(), false)
                    }
                }
            }
            Err(_) => {
                // If staging fails, fall through to fallback
                (Vec::new(), false)
            }
        }
    } else {
        (Vec::new(), false)
    };

    let pages = if used_llm && !pages.is_empty() {
        // Update manifest with new source + page records
        let (mut sources, mut page_records) = state.wiki.read_manifest().unwrap_or_default();
        sources.retain(|s| s.filename != body.filename);
        sources.push(SourceRecord {
            filename: body.filename.clone(),
            hash: source_hash.clone(),
            active: true,
            last_run_id: Some(run_id.clone()),
        });
        for page in &pages {
            page_records.retain(|r| r.slug != page.slug);
            page_records.push(PageRecord {
                slug: page.slug.clone(),
                page_type: page.page_type.clone(),
                path: format!("pages/{}/{}.md", page.page_type, page.slug),
                sources: vec![body.filename.clone()],
                last_run_id: run_id.clone(),
                managed_by: "source_ingest".to_string(),
            });
        }
        if let Err(e) = state.wiki.write_manifest(&sources, &page_records) {
            tracing::warn!("manifest write failed after ingest: {e}");
        }
        let _ = state.wiki.append_run(&RunRecord {
            run_id: run_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            model: body.model.clone(),
            prompt_hash,
            schema_hash,
            sources: vec![body.filename.clone()],
            status: "success".to_string(),
            pages_written: pages.iter().map(|p| p.slug.clone()).collect(),
        });

        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if let Err(e) = state.wiki.update_index() {
            tracing::error!("wiki index update failed after ingest: {e}");
        }
        if let Err(e) =
            state.wiki.append_log(&format!("## [{date}] ingest | {} — {} page(s) generated", body.filename, pages.len()))
        {
            tracing::warn!("wiki log append failed: {e}");
        }
        if let Err(e) = state.wiki.sync_to_memory(state.memory.as_ref()).await {
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
    } else {
        pages
    };
    let _ = pages; // suppress unused warning in fallback path

    // Fallback: write raw content as a single topic page.
    match state.wiki.ingest(&body.filename, &body.content) {
        Ok(page) => {
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            // Mark source as active in the manifest even on the fallback path.
            let source_hash = WikiManager::hash_content(&body.content);
            let (mut sources, page_records) = state.wiki.read_manifest().unwrap_or_default();
            sources.retain(|s| s.filename != body.filename);
            sources.push(crate::wiki::SourceRecord {
                filename: body.filename.clone(),
                hash: source_hash,
                active: true,
                last_run_id: None,
            });
            if let Err(e) = state.wiki.write_manifest(&sources, &page_records) {
                tracing::warn!("manifest write failed after fallback ingest: {e}");
            }
            if let Err(e) = state.wiki.update_index() {
                tracing::error!("wiki index update failed after fallback ingest: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("index update failed after writing 1 page: {e}"),
                        "pages_written": [page.slug]
                    })),
                )
                    .into_response();
            }
            if let Err(e) = state
                .wiki
                .append_log(&format!("## [{date}] ingest | {} — fallback single-page (no LLM)", body.filename))
            {
                tracing::warn!("wiki log append failed: {e}");
            }
            if let Err(e) = state.wiki.sync_to_memory(state.memory.as_ref()).await {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /wiki/sources — list all ingested source files with manifest metadata.
pub async fn list_wiki_sources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sources = match state.wiki.list_sources() {
        Ok(s) => s,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ).into_response(),
    };
    let (_, page_records) = state.wiki.read_manifest().unwrap_or_default();
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
    (StatusCode::OK, Json(serde_json::json!(items))).into_response()
}

/// GET /wiki/dir — return the absolute path to the wiki sources directory (for the Open Folder button).
pub async fn get_wiki_dir(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = state.wiki.wiki_dir().join("sources").to_string_lossy().into_owned();
    (StatusCode::OK, Json(serde_json::json!({"path": path}))).into_response()
}

/// POST /wiki/regenerate — clear all source-generated pages and recompile from all sources.
///
/// Preserves `user_query` pages. Uses staged builds for atomicity.
pub async fn regenerate_wiki(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegenerateRequest>,
) -> impl IntoResponse {
    use crate::wiki::{PageRecord, RunRecord, SourceRecord, WikiManager};

    // Read manifest (bootstrap from filesystem if absent)
    let (manifest_sources, manifest_pages) = state.wiki.read_manifest().unwrap_or_default();

    // Collect active source (filename, content) pairs
    let sources_list: Vec<(String, String)> = if manifest_sources.is_empty() {
        // Bootstrap: scan wiki/sources/ filesystem
        match state.wiki.list_sources() {
            Ok(records) => records
                .into_iter()
                .filter_map(|r| state.wiki.read_source(&r.filename).ok().map(|c| (r.filename, c)))
                .collect(),
            Err(_) => Vec::new(),
        }
    } else {
        manifest_sources
            .iter()
            .filter(|r| r.active)
            .filter_map(|r| state.wiki.read_source(&r.filename).ok().map(|c| (r.filename.clone(), c)))
            .collect()
    };

    // Identify source_ingest pages to be replaced (user_query pages are untouched)
    let managed_pages: Vec<PageRecord> = manifest_pages
        .iter()
        .filter(|r| r.managed_by == "source_ingest")
        .cloned()
        .collect();

    if sources_list.is_empty() {
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

    // Delete managed pages first (staged replace)
    if let Err(e) = state.wiki.delete_page_files(&managed_pages) {
        tracing::warn!("failed to delete managed pages before regenerate: {e}");
    }

    // Run compiler over all sources from a clean slate
    let llm_result = run_compiler(&state, &sources_list, body.model.as_deref()).await;
    let llm_pages = match llm_result {
        Ok(p) if !p.is_empty() => p,
        _ => {
            // Rollback: re-ingest sources with fallback (regenerate what we can)
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let _ = state.wiki.append_log(&format!(
                "## [{date}] regenerate | failed — LLM unavailable, no pages written"
            ));
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "LLM unavailable or returned no pages; managed pages were deleted. Re-ingest sources manually to recover."})),
            )
                .into_response();
        }
    };

    // Staged write
    let rebuild_dir = match state.wiki.begin_staged_build() {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("staged build init failed: {e}")})),
            )
                .into_response();
        }
    };
    for p in &llm_pages {
        if let Err(e) = state.wiki.write_staged_page(&rebuild_dir, &p.page_type, &p.slug, &p.content) {
            tracing::warn!("skipping page '{}': {e}", p.slug);
        }
    }
    let committed = match state.wiki.commit_staged_build(&rebuild_dir) {
        Ok(c) => c,
        Err(e) => {
            state.wiki.abort_staged_build(&rebuild_dir);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("staged build commit failed: {e}")})),
            )
                .into_response();
        }
    };

    // Write final pages to live pages/ and build page records
    let run_id = WikiManager::new_run_id();
    let prompt_hash = WikiManager::hash_content(&state.wiki.read_ingest_prompt().unwrap_or_default());
    let schema_hash = WikiManager::hash_content(
        &std::fs::read_to_string(state.wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default(),
    );
    let mut written_pages = Vec::new();
    let mut new_page_records: Vec<PageRecord> = Vec::new();
    for (ptype, slug) in &committed {
        if let Some(lp) = llm_pages.iter().find(|p| &p.slug == slug)
            && let Ok(page) = state.wiki.write_page(ptype, slug, &lp.content)
        {
            written_pages.push(page);
            new_page_records.push(PageRecord {
                slug: slug.clone(),
                page_type: ptype.clone(),
                path: format!("pages/{ptype}/{slug}.md"),
                sources: sources_list.iter().map(|(f, _)| f.clone()).collect(),
                last_run_id: run_id.clone(),
                managed_by: "source_ingest".to_string(),
            });
        }
    }

    // Update manifest: refresh source records + replace managed page records
    let new_source_records: Vec<SourceRecord> = sources_list
        .iter()
        .map(|(filename, content)| SourceRecord {
            filename: filename.clone(),
            hash: WikiManager::hash_content(content),
            active: true,
            last_run_id: Some(run_id.clone()),
        })
        .collect();
    let preserved_query_pages: Vec<PageRecord> = manifest_pages
        .into_iter()
        .filter(|r| r.managed_by != "source_ingest")
        .collect();
    let all_page_records: Vec<PageRecord> =
        new_page_records.into_iter().chain(preserved_query_pages).collect();
    if let Err(e) = state.wiki.write_manifest(&new_source_records, &all_page_records) {
        tracing::warn!("manifest write failed after regenerate: {e}");
    }
    let _ = state.wiki.append_run(&RunRecord {
        run_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
        model: body.model.clone(),
        prompt_hash,
        schema_hash,
        sources: sources_list.iter().map(|(f, _)| f.clone()).collect(),
        status: "success".to_string(),
        pages_written: written_pages.iter().map(|p| p.slug.clone()).collect(),
    });

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if let Err(e) = state.wiki.update_index() {
        tracing::warn!("wiki index update failed after regenerate: {e}");
    }
    if let Err(e) = state.wiki.append_log(&format!(
        "## [{date}] regenerate | {} source(s) → {} page(s)",
        sources_list.len(),
        written_pages.len()
    )) {
        tracing::warn!("wiki log append failed: {e}");
    }
    if let Err(e) = state.wiki.sync_to_memory(state.memory.as_ref()).await {
        tracing::warn!("wiki memory sync failed: {e}");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(RegenerateResponse {
            sources_processed: sources_list.len(),
            pages_generated: written_pages.len(),
            message: format!(
                "Regenerated from {} source(s); {} pages written.",
                sources_list.len(),
                written_pages.len()
            ),
        })),
    )
        .into_response()
}

/// POST /wiki/sources/:filename/regenerate — re-run the LLM compiler over a single source.
///
/// Deletes pages exclusively or partially derived from this source, then recompiles
/// from just this one source. Shared pages (those that list multiple contributing sources)
/// are deleted and rebuilt from this source only — run a full regeneration to restore
/// multi-source synthesis.
pub async fn regenerate_wiki_source(
    Path(filename): Path<String>,
    State(state): State<Arc<AppState>>,
    body: Option<Json<RegenerateRequest>>,
) -> impl IntoResponse {
    use crate::wiki::{PageRecord, RunRecord, SourceRecord, WikiManager};

    // Verify source exists
    let source_content = match state.wiki.read_source(&filename) {
        Ok(c) => c,
        Err(crate::error::ZeniiError::Validation(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("source '{}' not found", filename)})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let model = body.as_ref().and_then(|b| b.model.as_deref().map(String::from));

    // Find pages derived from this source
    let (manifest_sources, manifest_pages) = state.wiki.read_manifest().unwrap_or_default();
    let source_pages: Vec<PageRecord> = manifest_pages
        .iter()
        .filter(|p| p.sources.contains(&filename) && p.managed_by == "source_ingest")
        .cloned()
        .collect();

    // Remove them from memory
    for page in &source_pages {
        let key = format!("wiki:{}", page.slug);
        if let Err(e) = state.memory.forget(&key).await {
            tracing::warn!("failed to forget wiki page '{}' from memory: {e}", page.slug);
        }
    }

    // Delete page files
    if let Err(e) = state.wiki.delete_page_files(&source_pages) {
        tracing::warn!("failed to delete pages before per-source regenerate: {e}");
    }

    // Run compiler on just this one source
    let sources_list = vec![(filename.clone(), source_content)];
    let llm_result = run_compiler(&state, &sources_list, model.as_deref()).await;
    let llm_pages = match llm_result {
        Ok(p) if !p.is_empty() => p,
        _ => {
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let _ = state.wiki.append_log(&format!(
                "## [{date}] regenerate-source | {filename} — failed: LLM unavailable"
            ));
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "LLM unavailable or returned no pages"})),
            )
                .into_response();
        }
    };

    // Staged write
    let rebuild_dir = match state.wiki.begin_staged_build() {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("staged build init failed: {e}")})),
            )
                .into_response();
        }
    };
    for p in &llm_pages {
        if let Err(e) = state.wiki.write_staged_page(&rebuild_dir, &p.page_type, &p.slug, &p.content) {
            tracing::warn!("skipping page '{}': {e}", p.slug);
        }
    }
    let committed = match state.wiki.commit_staged_build(&rebuild_dir) {
        Ok(c) => c,
        Err(e) => {
            state.wiki.abort_staged_build(&rebuild_dir);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("staged build commit failed: {e}")})),
            )
                .into_response();
        }
    };

    // Write final pages and build page records
    let run_id = WikiManager::new_run_id();
    let prompt_hash = WikiManager::hash_content(&state.wiki.read_ingest_prompt().unwrap_or_default());
    let schema_hash = WikiManager::hash_content(
        &std::fs::read_to_string(state.wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default(),
    );
    let mut written_pages = Vec::new();
    let mut new_page_records: Vec<PageRecord> = Vec::new();
    for (ptype, slug) in &committed {
        if let Some(lp) = llm_pages.iter().find(|p| &p.slug == slug)
            && let Ok(page) = state.wiki.write_page(ptype, slug, &lp.content)
        {
            written_pages.push(page);
            new_page_records.push(PageRecord {
                slug: slug.clone(),
                page_type: ptype.clone(),
                path: format!("pages/{ptype}/{slug}.md"),
                sources: vec![filename.clone()],
                last_run_id: run_id.clone(),
                managed_by: "source_ingest".to_string(),
            });
        }
    }

    // Update manifest: keep existing records for other sources, replace records for this source
    let other_page_records: Vec<PageRecord> = manifest_pages
        .into_iter()
        .filter(|p| !p.sources.contains(&filename) || p.managed_by != "source_ingest")
        .collect();
    let all_page_records: Vec<PageRecord> =
        new_page_records.into_iter().chain(other_page_records).collect();

    // Update source record's last_run_id and hash
    let new_source_records: Vec<SourceRecord> = manifest_sources
        .into_iter()
        .map(|s| {
            if s.filename == filename {
                SourceRecord {
                    filename: s.filename,
                    hash: WikiManager::hash_content(&sources_list[0].1),
                    active: s.active,
                    last_run_id: Some(run_id.clone()),
                }
            } else {
                s
            }
        })
        .collect();

    if let Err(e) = state.wiki.write_manifest(&new_source_records, &all_page_records) {
        tracing::warn!("manifest write failed after per-source regenerate: {e}");
    }
    let _ = state.wiki.append_run(&RunRecord {
        run_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
        model: model.clone(),
        prompt_hash,
        schema_hash,
        sources: vec![filename.clone()],
        status: "success".to_string(),
        pages_written: written_pages.iter().map(|p| p.slug.clone()).collect(),
    });

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if let Err(e) = state.wiki.update_index() {
        tracing::warn!("wiki index update failed after per-source regenerate: {e}");
    }
    let _ = state.wiki.append_log(&format!(
        "## [{date}] regenerate-source | {filename} → {} page(s)",
        written_pages.len()
    ));
    if let Err(e) = state.wiki.sync_to_memory(state.memory.as_ref()).await {
        tracing::warn!("wiki memory sync failed: {e}");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(RegenerateResponse {
            sources_processed: 1,
            pages_generated: written_pages.len(),
            message: format!("Regenerated from '{}'; {} pages written.", filename, written_pages.len()),
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

    let filename = filename.unwrap_or_else(|| "upload.bin".to_string());

    // Write bytes to a temp file for conversion
    let tmp_path = std::env::temp_dir().join(&filename);
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

    // Feed into standard ingest pipeline via the existing handler body
    let body = IngestRequest {
        content,
        filename,
        model,
    };

    ingest_wiki_source(State(state), Json(body)).await.into_response()
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

    // Verify source exists
    if state.wiki.read_source(&filename).is_err() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("source '{filename}' not found")})),
        )
            .into_response();
    }

    let (manifest_sources, manifest_pages) = state.wiki.read_manifest().unwrap_or_default();

    // Find pages contributed to by this source
    let affected: Vec<&PageRecord> = manifest_pages
        .iter()
        .filter(|r| r.managed_by == "source_ingest" && r.sources.contains(&filename))
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

    // Delete exclusive pages from disk and memory
    if let Err(e) = state.wiki.delete_page_files(&exclusive) {
        tracing::warn!("failed to delete exclusive pages for source '{filename}': {e}");
    }
    for slug in &deleted_slugs {
        let key = format!("wiki:{slug}");
        if let Err(e) = state.memory.forget(&key).await {
            tracing::warn!("memory forget failed for '{key}': {e}");
        }
    }

    // Remove source from frontmatter of shared pages
    for r in &shared {
        if let Err(e) = state.wiki.remove_source_from_page(r, &filename) {
            tracing::warn!("failed to remove source from page '{}': {e}", r.slug);
        }
    }

    // Delete the raw source file
    if let Err(e) = state.wiki.delete_source_file(&filename) {
        tracing::warn!("failed to delete source file '{filename}': {e}");
    }

    // Rebuild shared pages from their remaining contributing sources (if any)
    let mut rebuilt_pages = Vec::new();
    if !shared.is_empty() {
        let remaining_sources: Vec<(String, String)> = shared
            .iter()
            .flat_map(|r| r.sources.iter().filter(|s| *s != &filename))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .filter_map(|f| state.wiki.read_source(f).ok().map(|c| (f.clone(), c)))
            .collect();

        if !remaining_sources.is_empty()
            && let Ok(llm_pages) =
                run_compiler(&state, &remaining_sources, params.model.as_deref()).await
        {
            let rebuild_dir = state.wiki.begin_staged_build().unwrap_or_else(|_| {
                state.wiki.wiki_dir().join(".rebuild")
            });
            for p in &llm_pages {
                if rebuilt_slugs.contains(&p.slug) {
                    let _ = state.wiki.write_staged_page(&rebuild_dir, &p.page_type, &p.slug, &p.content);
                }
            }
            if state.wiki.commit_staged_build(&rebuild_dir).is_ok() {
                for p in &llm_pages {
                    if rebuilt_slugs.contains(&p.slug)
                        && let Ok(page) = state.wiki.write_page(&p.page_type, &p.slug, &p.content)
                    {
                        rebuilt_pages.push(page);
                    }
                }
            } else {
                state.wiki.abort_staged_build(&rebuild_dir);
            }
        }
    }

    // Update manifest
    let run_id = WikiManager::new_run_id();
    let new_sources: Vec<_> = manifest_sources.into_iter().filter(|s| s.filename != filename).collect();
    let new_pages: Vec<PageRecord> = manifest_pages
        .into_iter()
        .filter(|r| !deleted_slugs.contains(&r.slug))
        .map(|mut r| {
            if rebuilt_slugs.contains(&r.slug) {
                r.sources.retain(|s| s != &filename);
                r.last_run_id = run_id.clone();
            }
            r
        })
        .collect();
    if let Err(e) = state.wiki.write_manifest(&new_sources, &new_pages) {
        tracing::warn!("manifest write failed after delete-source: {e}");
    }
    let _ = state.wiki.append_run(&RunRecord {
        run_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
        model: params.model.clone(),
        prompt_hash: WikiManager::hash_content(&state.wiki.read_ingest_prompt().unwrap_or_default()),
        schema_hash: WikiManager::hash_content(
            &std::fs::read_to_string(state.wiki.wiki_dir().join("SCHEMA.md")).unwrap_or_default(),
        ),
        sources: vec![filename.clone()],
        status: "success".to_string(),
        pages_written: rebuilt_pages.iter().map(|p| p.slug.clone()).collect(),
    });

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if let Err(e) = state.wiki.update_index() {
        tracing::warn!("wiki index update failed after delete-source: {e}");
    }
    if let Err(e) = state.wiki.append_log(&format!(
        "## [{date}] delete-source | {filename} — {} deleted, {} rebuilt",
        deleted_slugs.len(),
        rebuilt_pages.len()
    )) {
        tracing::warn!("wiki log append failed: {e}");
    }
    if let Err(e) = state.wiki.sync_to_memory(state.memory.as_ref()).await {
        tracing::warn!("wiki memory sync failed: {e}");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(DeleteSourceResponse {
            filename,
            deleted_pages: deleted_slugs,
            rebuilt_pages: rebuilt_pages.iter().map(|p| p.slug.clone()).collect(),
            message: "Source deleted and wiki updated.".to_string(),
        })),
    )
        .into_response()
}

/// Shared compiler pipeline: call the LLM to generate wiki pages from a list of sources.
///
/// Reads `INGEST_PROMPT.md` and `SCHEMA.md` at runtime (no hardcoded prompts).
/// Returns `Ok(pages)` on success or `Err(())` when the agent or JSON parsing fails.
async fn run_compiler(
    state: &AppState,
    sources: &[(String, String)],
    model: Option<&str>,
) -> Result<Vec<LlmPage>, ()> {
    use crate::ai::resolve_agent;

    if sources.is_empty() {
        return Ok(Vec::new());
    }

    // Read INGEST_PROMPT.md (user-editable) and SCHEMA.md at runtime.
    let ingest_prompt = state.wiki.read_ingest_prompt().unwrap_or_default();
    let schema = std::fs::read_to_string(state.wiki.wiki_dir().join("SCHEMA.md"))
        .unwrap_or_default();

    let system_prompt = format!("{ingest_prompt}\n\n{schema}");

    let mut all_pages: Vec<LlmPage> = Vec::new();

    for (filename, content) in sources {
        // Provide existing wiki context so LLM can update/merge pages
        let current_index = state.wiki.read_index().unwrap_or_default();
        let user_prompt = if all_pages.is_empty()
            && (current_index.is_empty() || current_index.contains("No pages yet"))
        {
            format!("Filename: {filename}\n\nContent:\n{content}")
        } else {
            let summaries = state
                .wiki
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
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Filename: {filename}\n\nCurrent wiki index:\n{current_index}\n\nPage summaries:\n{summaries}\n\nNew source content:\n{content}"
            )
        };

        let agent = resolve_agent(model, state, None, Some(&system_prompt), "wiki")
            .await
            .map_err(|_| ())?;

        let response = agent.prompt(&user_prompt).await.map_err(|_| ())?;

        // Strip optional markdown code fences
        let raw = response
            .output
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(pages) = serde_json::from_str::<Vec<LlmPage>>(raw) {
            all_pages.extend(pages);
        }
    }

    if all_pages.is_empty() { Err(()) } else { Ok(all_pages) }
}

/// POST /wiki/sync — sync compiled wiki pages into the memory store.
pub async fn sync_wiki_to_memory(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.wiki.sync_to_memory(state.memory.as_ref()).await {
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
    match state.wiki.graph() {
        Ok(graph) => (StatusCode::OK, Json(serde_json::json!(graph))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
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

    // Read the current index and all page bodies as context.
    let index = match state.wiki.read_index() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let pages = match state.wiki.list_pages() {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let schema = std::fs::read_to_string(state.wiki.wiki_dir().join("SCHEMA.md"))
        .unwrap_or_default();

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
                .into_response()
        }
    };

    let response = match agent.prompt(&body.question).await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let answer = response.output;
    let citations = extract_wikilink_citations(&answer);

    // Optionally save the answer as a query page.
    let saved_page = if body.save.unwrap_or(false) {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let slug = slugify_question(&body.question);
        let first_line = answer.lines().next().unwrap_or("").to_string();
        let content = format!(
            "---\ntitle: \"{}\"\ntype: query\ntags: []\nsources: []\nupdated: {date}\n---\n\n## TLDR\n{first_line}\n\n## Body\n{answer}\n",
            body.question,
        );
        if let Ok(page) = state.wiki.write_page("queries", &slug, &content) {
            if let Err(e) = state.wiki.update_index() {
                tracing::warn!("wiki index update failed after saving query page: {e}");
            }
            Some(page)
        } else {
            None
        }
    } else {
        None
    };

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let log_entry = format!("## [{date}] query | {}", body.question);
    if let Err(e) = state.wiki.append_log(&log_entry) {
        tracing::warn!("wiki log append failed after query: {e}");
    }

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

    let issues = match state.wiki.lint() {
        Ok(i) => i,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let (issues, fixed) = if auto_fix && !issues.is_empty() {
        match state.wiki.lint_fix(&issues) {
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
        let mut counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for issue in &issues {
            *counts.entry(issue.kind.as_str()).or_insert(0) += 1;
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
    if let Err(e) = state.wiki.append_log(&log_entry) {
        tracing::warn!("wiki log append failed after lint: {e}");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(LintResponse { issues, fixed, summary })),
    )
        .into_response()
}

/// GET /wiki/prompt — read current INGEST_PROMPT.md content.
pub async fn get_wiki_prompt(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.wiki.read_ingest_prompt() {
        Ok(content) => (StatusCode::OK, Json(PromptResponse { content })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// PUT /wiki/prompt — write new content to INGEST_PROMPT.md.
pub async fn set_wiki_prompt(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetPromptRequest>,
) -> impl IntoResponse {
    match state.wiki.set_prompt(&body.content) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(ZeniiError::Validation(msg)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error_code": "ZENII_VALIDATION",
                "message": msg
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /wiki/sources — delete all source files, clear manifest source records, and remove ingest pages.
pub async fn delete_all_wiki_sources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (deleted, ingest_pages) = match state.wiki.delete_all_sources() {
        Ok(r) => r,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ).into_response(),
    };

    // Remove deleted pages from the memory store
    for page in &ingest_pages {
        let key = format!("wiki:{}", page.slug);
        if let Err(e) = state.memory.forget(&key).await {
            tracing::warn!("failed to forget wiki page '{}' from memory: {e}", page.slug);
        }
    }

    // Update index.md to reflect only remaining (user_query) pages
    if let Err(e) = state.wiki.update_index() {
        tracing::warn!("wiki index update failed after delete-all-sources: {e}");
    }

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let _ = state.wiki.append_log(&format!(
        "## [{date}] delete-all-sources | {deleted} source(s) deleted, {} pages removed",
        ingest_pages.len()
    ));

    (
        StatusCode::OK,
        Json(DeleteSourcesResponse {
            message: format!("Deleted {deleted} source files and {} pages", ingest_pages.len()),
            deleted,
        }),
    )
        .into_response()
}

/// DELETE /wiki/pages — delete all wiki pages and reset index.md.
pub async fn delete_wiki_pages(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.wiki.delete_all_pages() {
        Ok(deleted) => (
            StatusCode::OK,
            Json(DeletePagesResponse {
                message: format!("Deleted {deleted} wiki pages"),
                deleted,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
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
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has",
        "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall",
        "can", "what", "how", "why", "when", "where", "who", "which", "that", "this", "these",
        "those", "in", "on", "at", "to", "for", "of", "with", "by", "from", "and", "or", "not",
        "it", "its", "i", "my", "me", "you", "your", "we", "our", "they", "their",
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
            .route(
                "/wiki/sources/{filename}",
                delete(delete_wiki_source),
            )
            .route("/wiki/dir", get(get_wiki_dir))
            .route(
                "/wiki/prompt",
                get(get_wiki_prompt).put(set_wiki_prompt),
            )
            .route("/wiki/pages", delete(delete_wiki_pages))
            .route("/wiki/regenerate", post(regenerate_wiki))
            .route("/wiki/{slug}", get(get_wiki_page))
            .with_state(state)
    }

    /// H1: GET /wiki with no pages → 200 with empty array `[]`
    #[tokio::test]
    async fn wiki_list_empty_returns_200_empty_array() {
        let (_dir, state) = test_state().await;

        let req = Request::builder().uri("/wiki").body(Body::empty()).unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(pages.is_empty(), "expected empty array");
    }

    /// H2: GET /wiki → 200 with a JSON array (even if empty during stub phase)
    #[tokio::test]
    async fn wiki_list_returns_pages() {
        let (_dir, state) = test_state().await;

        let req = Request::builder().uri("/wiki").body(Body::empty()).unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        // Body must be a valid JSON array
        let _pages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
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
        let bytes1 = axum::body::to_bytes(resp1.into_body(), 16 * 1024).await.unwrap();
        let val1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
        let slug = val1["primary_slug"].as_str().unwrap_or("my-doc").to_string();

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
        let list_bytes = axum::body::to_bytes(list_resp.into_body(), 64 * 1024).await.unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_slice(&list_bytes).unwrap();
        let count = pages.iter().filter(|p| p["slug"].as_str() == Some(&slug)).count();
        assert_eq!(count, 1, "slug '{slug}' must appear exactly once after two ingests");
    }

    /// H14: When index.md is read-only, update_index() fails and ingest must return
    /// non-200 with a `pages_written` field in the error body (unix-only).
    #[cfg(unix)]
    #[tokio::test]
    async fn wiki_ingest_returns_500_when_index_update_fails() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, state) = test_state().await;

        // Make index.md read-only so update_index() cannot overwrite it.
        let index_path = state.wiki.wiki_dir().join("index.md");
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

        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024).await.unwrap();
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
        assert!(!results.is_empty(), "search for 'foxglove' must return at least one result");
    }
}
