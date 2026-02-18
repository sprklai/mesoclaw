//! Tauri IPC commands for agent session management.
//!
//! Each command builds an [`AgentLoop`] on-the-fly from managed app state,
//! runs a single agent turn, and returns the final response.  Cancellation
//! is tracked via a shared `SessionCancelMap`.

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use diesel::prelude::*;
use tauri::State;
use uuid::Uuid;

use crate::{
    agent::loop_::{AgentConfig, AgentLoop},
    ai::{
        provider::LLMProvider,
        providers::openai_compatible::{OpenAICompatibleConfig, OpenAICompatibleProvider},
    },
    config::app_identity::KEYCHAIN_SERVICE,
    database::{
        DbPool,
        schema::{ai_models, ai_providers},
        models::ai_provider::{AIModel, AIProvider},
    },
    event_bus::{AppEvent, EventBus},
    identity::IdentityLoader,
    security::SecurityPolicy,
    tools::ToolRegistry,
};

// ─── Cancellation map ─────────────────────────────────────────────────────────

/// Shared map from `session_id → cancellation flag`, managed in Tauri state.
/// A `true` value tells the session to abort at the next safe boundary.
pub type SessionCancelMap =
    Arc<Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>>>;

// ─── Provider resolution ──────────────────────────────────────────────────────

/// Resolve the active LLM provider from the database + OS keyring.
///
/// Lookup order:
/// 1. `settings.default_provider_id` → concrete provider row
/// 2. First `is_active = 1` provider as fallback
/// 3. Read API key from OS keyring using format `api_key:{provider_id}`
fn resolve_active_provider(pool: &DbPool) -> Result<Arc<dyn LLMProvider>, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;

    // 1. Read the preferred provider id from settings (column is nullable).
    use crate::database::schema::settings as s;
    let preferred_id: Option<String> = s::table
        .select(s::default_provider_id)
        .first::<Option<String>>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to query settings: {e}"))?
        .flatten();

    // 2. Find the provider row.
    let provider: AIProvider = if let Some(ref pid) = preferred_id {
        ai_providers::table
            .filter(ai_providers::id.eq(pid))
            .first::<AIProvider>(&mut conn)
            .map_err(|e| format!("Preferred provider '{pid}' not found: {e}"))?
    } else {
        ai_providers::table
            .filter(ai_providers::is_active.eq(1))
            .first::<AIProvider>(&mut conn)
            .map_err(|_| {
                "No active LLM provider configured. Open Settings → Providers.".to_string()
            })?
    };

    // 3. Pick the first active model for this provider.
    let model: AIModel = ai_models::table
        .filter(ai_models::provider_id.eq(&provider.id))
        .filter(ai_models::is_active.eq(1))
        .first::<AIModel>(&mut conn)
        .map_err(|_| format!("No active model for provider '{}'.", provider.id))?;

    // 4. Retrieve API key from OS keyring.
    let api_key: String = if provider.requires_api_key == 0 {
        "local".to_string()
    } else {
        let key_name = format!("api_key:{}", provider.id);
        keyring::Entry::new(KEYCHAIN_SERVICE, &key_name)
            .map_err(|e| format!("Keyring entry error: {e}"))?
            .get_password()
            .map_err(|_| format!(
                "No API key stored for '{}'. Open Settings → Providers and save your key.",
                provider.id
            ))?
    };

    // 5. Build the provider instance.
    let cfg = OpenAICompatibleConfig::with_model(&api_key, &provider.base_url, &model.model_id);
    let instance = OpenAICompatibleProvider::new(cfg, &provider.id)
        .map_err(|e| format!("Failed to create provider: {e}"))?;

    Ok(Arc::new(instance))
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Start an agent session for a user message.
///
/// Resolves the active LLM provider from the database and OS keyring,
/// constructs an [`AgentLoop`] from managed app state, and runs a single
/// multi-turn agent turn.  Intermediate tool events are emitted on the
/// [`EventBus`] and forwarded to the frontend via [`TauriBridge`].
///
/// Returns the agent's final text response.
#[tauri::command]
pub async fn start_agent_session_command(
    message: String,
    pool: State<'_, DbPool>,
    tool_registry: State<'_, Arc<ToolRegistry>>,
    security_policy: State<'_, Arc<SecurityPolicy>>,
    event_bus: State<'_, Arc<dyn EventBus>>,
    identity_loader: State<'_, IdentityLoader>,
    cancel_map: State<'_, SessionCancelMap>,
) -> Result<String, String> {
    let session_id = Uuid::new_v4().to_string();

    // Register a cancellation flag for this session.
    let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut map = cancel_map.lock().map_err(|e| format!("lock: {e}"))?;
        map.insert(session_id.clone(), Arc::clone(&flag));
    }

    // Resolve LLM provider from DB + keyring.
    let provider = resolve_active_provider(&pool)?;

    // Clone Arc handles out of Tauri State wrappers.
    let registry = Arc::clone(&*tool_registry);
    let policy = Arc::clone(&*security_policy);
    let bus = Arc::clone(&*event_bus);

    // Build system prompt from identity files.
    let system_prompt = identity_loader.build_system_prompt();

    // Construct and run the agent loop.
    let agent = AgentLoop::new(provider, registry, policy, Some(bus.clone()), AgentConfig::default());
    let result = agent.run(&system_prompt, &message).await;

    // Remove the cancellation entry when done.
    {
        let mut map = cancel_map.lock().map_err(|e| format!("lock: {e}"))?;
        map.remove(&session_id);
    }

    let response_text = result?;

    // Emit AgentComplete so EventBus subscribers (e.g. channels) know we're done.
    let _ = bus.publish(AppEvent::AgentComplete {
        session_id,
        message: response_text.clone(),
    });

    Ok(response_text)
}

/// Cancel a running agent session by session id.
///
/// Sets the cancellation flag so the agent loop will abort at the next
/// iteration boundary.  Returns immediately; the session winds down shortly.
#[tauri::command]
pub async fn cancel_agent_session_command(
    session_id: String,
    cancel_map: State<'_, SessionCancelMap>,
) -> Result<(), String> {
    let map = cancel_map.lock().map_err(|e| format!("lock: {e}"))?;
    match map.get(&session_id) {
        Some(flag) => {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        None => Err(format!("Session '{}' not found or already complete.", session_id)),
    }
}
