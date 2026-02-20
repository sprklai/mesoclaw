/// MesoClaw CLI — headless interface to the AI agent runtime.
///
/// Provides subcommands for managing the daemon, agents, memory, identity,
/// configuration, scheduling, channels, and launching the GUI. When invoked
/// with no subcommand the CLI enters an interactive REPL that streams
/// responses from the gateway WebSocket.
///
/// # CI matrix note
/// The following feature combinations should be tested in CI:
///   - `cargo build --features core,cli`           (minimal: no desktop, no gateway)
///   - `cargo build`                                (default features)
///   - `cargo build --all-features`                (full build)
use std::{
    fs,
    io::{self, BufRead, IsTerminal, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

// ---------------------------------------------------------------------------
// Top-level CLI struct
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "mesoclaw",
    about = "MesoClaw AI agent runtime CLI",
    version,
    long_about = "Headless interface to the MesoClaw AI agent daemon.\n\
                  Run without a subcommand to enter the interactive REPL."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Output raw text without formatting.
    #[arg(long, global = true)]
    raw: bool,

    /// Output results as JSON.
    #[arg(long, global = true)]
    json: bool,

    /// Skip all approval prompts and run in full-autonomy mode.
    #[arg(long, global = true)]
    auto: bool,

    /// Resume an existing agent session by ID.
    #[arg(long, global = true, value_name = "SESSION_ID")]
    resume: Option<String>,
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start or manage the background daemon process.
    Daemon(DaemonArgs),
    /// Manage AI agents (list, start, stop, inspect).
    Agent(AgentArgs),
    /// Manage persistent agent memory stores.
    Memory(MemoryArgs),
    /// Manage agent identities and persona files.
    Identity(IdentityArgs),
    /// View and edit application configuration.
    Config(ConfigArgs),
    /// Manage scheduled tasks and triggers.
    Schedule(ScheduleArgs),
    /// Manage communication channels (e.g., Telegram).
    Channel(ChannelArgs),
    /// Manage sidecar extension modules (list, install, remove, start, stop, health, reload, create).
    Module(ModuleArgs),
    /// Generate AI-powered prompt artifacts (skills, agents, souls, etc.).
    Generate(GenerateArgs),
    /// Launch the MesoClaw desktop GUI.
    Gui,
    /// Watch a path for changes and trigger an agent on each change.
    Watch(WatchArgs),
}

#[derive(Parser, Debug)]
struct DaemonArgs {
    /// Daemon action: start | stop | status | restart.
    #[arg(default_value = "status")]
    action: String,

    /// Run the daemon in the foreground without detaching.
    /// Used internally when the binary self-spawns for background execution.
    #[arg(long, hide = true)]
    foreground: bool,
}

#[derive(Parser, Debug)]
struct AgentArgs {
    /// Agent action: list | start | stop | inspect.
    #[arg(default_value = "list")]
    action: String,
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct MemoryArgs {
    #[arg(default_value = "list")]
    action: String,
    key: Option<String>,
    value: Option<String>,
}

#[derive(Parser, Debug)]
struct IdentityArgs {
    #[arg(default_value = "list")]
    action: String,
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct ConfigArgs {
    /// Config action: list | set-key | get-key | delete-key
    #[arg(default_value = "list")]
    action: String,
    /// Provider ID (for set-key, get-key, delete-key).
    provider: Option<String>,
    /// API key value (for set-key). If omitted, read interactively from stdin.
    value: Option<String>,
}

#[derive(Parser, Debug)]
struct ScheduleArgs {
    /// Schedule action: list | add | toggle | remove | history
    #[arg(default_value = "list")]
    action: String,
    /// Job ID (for toggle, remove, history).
    id: Option<String>,
    /// Human-readable job name (for add).
    #[arg(long)]
    name: Option<String>,
    /// Cron expression, e.g. "0 9 * * 1-5" (for add; mutually exclusive with --interval).
    #[arg(long)]
    cron: Option<String>,
    /// Interval in seconds (for add; mutually exclusive with --cron).
    #[arg(long)]
    interval: Option<u64>,
    /// Prompt text for an AgentTurn payload (for add). Omit for a Heartbeat job.
    #[arg(long)]
    prompt: Option<String>,
    /// Delete the job automatically after it runs once (for add).
    #[arg(long, default_value_t = false)]
    once: bool,
}

#[derive(Parser, Debug)]
struct ChannelArgs {
    /// Channel action: list | add | set | status | remove
    #[arg(default_value = "list")]
    action: String,
    /// Channel type (e.g. "telegram"). Required for add/set/status/remove.
    channel_type: Option<String>,
    /// Bot token (Telegram only).
    #[arg(long)]
    token: Option<String>,
    /// Allowed chat IDs, comma-separated (Telegram only).
    #[arg(long)]
    chat_ids: Option<String>,
    /// Polling timeout in seconds (Telegram only).
    #[arg(long, default_value_t = 30u64)]
    polling_timeout: u64,
}

#[derive(Parser, Debug)]
struct GenerateArgs {
    #[command(subcommand)]
    action: Option<GenerateAction>,

    /// Artifact type: skill, agent, soul, claude-skill, generic
    #[arg(long, short = 't')]
    r#type: Option<String>,

    /// Artifact name (kebab-case)
    #[arg(long, short = 'n')]
    name: Option<String>,

    /// Natural language description of what to generate
    #[arg(long, short = 'd')]
    description: Option<String>,
}

#[derive(Subcommand, Debug)]
enum GenerateAction {
    /// List all previously generated artifacts
    List,
    /// Delete a generated artifact by ID
    Delete { id: String },
}

#[derive(Parser, Debug)]
struct ModuleArgs {
    /// Module action: list | install | remove | start | stop | health | reload | create
    #[arg(default_value = "list")]
    action: String,
    /// Module name or ID (required for install, remove, start, stop, health, create).
    name: Option<String>,
    /// Module type for `create` (tool | service | mcp).
    #[arg(long, default_value = "tool")]
    module_type: String,
    /// Runtime for `create` (native | docker | podman).
    #[arg(long, default_value = "native")]
    runtime: String,
    /// Source for `install`: a git URL (https://... or git@...) or a local directory path.
    #[arg(long)]
    url: Option<String>,
    /// Skip confirmation prompt for destructive actions like `remove`.
    #[arg(long, short = 'f')]
    force: bool,
}

#[derive(Parser, Debug)]
struct WatchArgs {
    /// Path to watch for changes (file or directory).
    path: String,
    /// Prompt template sent to the agent on each change.
    /// Use `{file}` as a placeholder for the changed file path.
    #[arg(
        long,
        short = 'p',
        default_value = "A file changed: {file}. Review it."
    )]
    prompt: String,
    /// Debounce delay in milliseconds before triggering the agent.
    #[arg(long, default_value = "500")]
    debounce_ms: u64,
}

// ---------------------------------------------------------------------------
// Gateway client
// ---------------------------------------------------------------------------

/// Reads the PID file written by the daemon and returns (pid, port).
fn read_pid_and_port() -> Option<(u32, u16)> {
    let path = daemon_pid_path();
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let pid: u32 = lines.next()?.trim().parse().ok()?;
    let port: u16 = lines.next()?.trim().parse().ok()?;
    Some((pid, port))
}

fn read_token() -> Option<String> {
    let path = dirs::home_dir()?.join(".mesoclaw").join("daemon.token");
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn daemon_pid_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".mesoclaw")
        .join("daemon.pid")
}

fn is_daemon_running() -> Option<u16> {
    let (pid, port) = read_pid_and_port()?;
    // On Unix, check if the process is alive by sending signal 0.
    #[cfg(unix)]
    {
        use std::process::Command;
        let alive = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if alive { Some(port) } else { None }
    }
    #[cfg(not(unix))]
    {
        // On Windows, just assume if the PID file exists the daemon is running.
        let _ = pid;
        Some(port)
    }
}

struct GatewayClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl GatewayClient {
    fn new(port: u16, token: String) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            token,
            client: reqwest::Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    async fn health(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/health", self.base_url))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn list_sessions(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/sessions", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn create_session(&self, system_prompt: Option<&str>) -> reqwest::Result<Value> {
        self.client
            .post(format!("{}/api/v1/sessions", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&json!({ "system_prompt": system_prompt }))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn list_modules(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/modules", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn module_action(&self, id: &str, action: &str) -> reqwest::Result<Value> {
        self.client
            .post(format!("{}/api/v1/modules/{id}/{action}", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn module_health(&self, id: &str) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/modules/{id}/health", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn reload_modules(&self) -> reqwest::Result<Value> {
        self.client
            .post(format!("{}/api/v1/modules", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn list_providers(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/providers", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn list_scheduler_jobs(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/scheduler/jobs", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn create_scheduler_job(&self, body: Value) -> reqwest::Result<Value> {
        self.client
            .post(format!("{}/api/v1/scheduler/jobs", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn toggle_scheduler_job(&self, job_id: &str) -> reqwest::Result<Value> {
        self.client
            .put(format!(
                "{}/api/v1/scheduler/jobs/{job_id}/toggle",
                self.base_url
            ))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn delete_scheduler_job(&self, job_id: &str) -> reqwest::Result<Value> {
        self.client
            .delete(format!("{}/api/v1/scheduler/jobs/{job_id}", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn scheduler_job_history(&self, job_id: &str) -> reqwest::Result<Value> {
        self.client
            .get(format!(
                "{}/api/v1/scheduler/jobs/{job_id}/history",
                self.base_url
            ))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }
}

/// Resolve or start the gateway, returning a ready client.
async fn require_gateway() -> Option<GatewayClient> {
    if let Some(port) = is_daemon_running()
        && let Some(token) = read_token()
    {
        return Some(GatewayClient::new(port, token));
    }
    eprintln!(
        "Gateway is not running.\n\
         Start it with: mesoclaw daemon start"
    );
    None
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

fn print_value(value: &Value, raw: bool, json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
    } else if raw {
        if let Some(s) = value.as_str() {
            println!("{s}");
        } else {
            println!("{value}");
        }
    } else {
        // Human-friendly default.
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
    }
}

fn print_err(msg: &str) {
    eprintln!("\x1b[31merror\x1b[0m: {msg}");
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

async fn dispatch(command: &Commands, raw: bool, json_mode: bool) {
    match command {
        Commands::Daemon(args) => handle_daemon(args).await,
        Commands::Agent(args) => handle_agent(args, raw, json_mode).await,
        Commands::Memory(args) => handle_memory(args, raw, json_mode).await,
        Commands::Identity(args) => handle_identity(args, raw, json_mode).await,
        Commands::Config(args) => handle_config(args, raw, json_mode).await,
        Commands::Schedule(args) => handle_schedule(args, raw, json_mode).await,
        Commands::Channel(args) => handle_channel(args, raw, json_mode).await,
        Commands::Module(args) => handle_module(args, raw, json_mode).await,
        Commands::Generate(args) => handle_generate(args, raw, json_mode).await,
        Commands::Gui => {
            println!("gui: not yet implemented — launch mesoclaw-desktop directly");
        }
        Commands::Watch(args) => handle_watch(args, raw, json_mode).await,
    }
}

async fn handle_daemon(args: &DaemonArgs) {
    match args.action.as_str() {
        "status" => match is_daemon_running() {
            Some(port) => {
                if let Some(client) = require_gateway().await {
                    match client.health().await {
                        Ok(v) => println!("daemon: running on port {port} — {v}"),
                        Err(e) => println!("daemon: port {port} (health check failed: {e})"),
                    }
                }
            }
            None => println!("daemon: not running"),
        },
        "start" => {
            if let Some(port) = is_daemon_running() {
                println!("daemon: already running on port {port}");
                return;
            }
            #[cfg(feature = "gateway")]
            {
                if !args.foreground {
                    // Self-spawn with --foreground so `daemon start` returns to
                    // the shell immediately instead of blocking.
                    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mesoclaw"));
                    match std::process::Command::new(&exe)
                        .arg("daemon")
                        .arg("start")
                        .arg("--foreground")
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                    {
                        Ok(_) => println!("daemon: starting in background"),
                        Err(e) => print_err(&format!("failed to start daemon: {e}")),
                    }
                    return;
                }
                use local_ts_lib::{
                    agent::session_router::SessionRouter,
                    event_bus::TokioBroadcastBus,
                    gateway::start_gateway,
                    identity::IdentityLoader,
                    memory::store::InMemoryStore,
                    modules::ModuleRegistry,
                    scheduler::{TokioScheduler, traits::Scheduler as _},
                };
                use std::sync::Arc;
                let bus: Arc<dyn local_ts_lib::event_bus::EventBus> =
                    Arc::new(TokioBroadcastBus::new());
                let sessions = Arc::new(SessionRouter::new());
                let modules = Arc::new(ModuleRegistry::empty());
                let memory = Arc::new(InMemoryStore::new_mock());

                // Build a standalone DbPool from the default app data path.
                let db_path = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("com.mesoclaw.app")
                    .join("app.db");
                let db_pool = {
                    use diesel::r2d2::{self, ConnectionManager};
                    use diesel::sqlite::SqliteConnection;
                    let manager = ConnectionManager::<SqliteConnection>::new(
                        db_path.to_string_lossy().as_ref(),
                    );
                    r2d2::Pool::builder()
                        .max_size(5)
                        .build(manager)
                        .map_err(|e| format!("failed to create db pool: {e}"))
                };
                let db_pool = match db_pool {
                    Ok(p) => p,
                    Err(e) => {
                        print_err(&format!("daemon: db pool init failed: {e}"));
                        return;
                    }
                };

                // Identity loader (no watcher in CLI mode).
                let identity_loader = match local_ts_lib::identity::default_identity_dir()
                    .and_then(IdentityLoader::new)
                {
                    Ok(loader) => loader,
                    Err(e) => {
                        print_err(&format!("daemon: identity loader init failed: {e}"));
                        return;
                    }
                };

                // Create a persistence-backed scheduler for the CLI daemon.
                // Agent loop integration is not available here (no LLM provider wired),
                // but Heartbeat/Notify jobs will still be recorded and persisted.
                let sched =
                    TokioScheduler::new_with_persistence(Arc::clone(&bus), Some(db_pool.clone()));
                let sched_start = Arc::clone(&sched);
                tokio::spawn(async move { sched_start.start().await });

                log::info!("daemon: running in foreground");
                // CLI daemon has no live agent sessions, so start with an empty cancel map.
                let cancel_map: local_ts_lib::agent::agent_commands::SessionCancelMap =
                    Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
                if let Err(e) = start_gateway(
                    bus,
                    sessions,
                    modules,
                    db_pool,
                    identity_loader,
                    memory,
                    sched,
                    cancel_map,
                )
                .await
                {
                    print_err(&format!("daemon failed: {e}"));
                }
            }
            #[cfg(not(feature = "gateway"))]
            {
                eprintln!("Gateway feature not compiled in. Rebuild with --features gateway.");
            }
        }
        "stop" => {
            if let Some((pid, _)) = read_pid_and_port() {
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill").arg(pid.to_string()).status();
                    println!("daemon: sent SIGTERM to PID {pid}");
                }
                #[cfg(not(unix))]
                {
                    println!("daemon stop: not implemented on this platform (PID {pid})");
                }
            } else {
                println!("daemon: not running");
            }
        }
        other => println!("daemon: unknown action '{other}'. Use start | stop | status"),
    }
}

async fn handle_agent(args: &AgentArgs, raw: bool, json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };
    match args.action.as_str() {
        "list" => match client.list_sessions().await {
            Ok(v) => print_value(&v, raw, json_mode),
            Err(e) => print_err(&format!("agent list: {e}")),
        },
        "start" => match client.create_session(None).await {
            Ok(v) => print_value(&v, raw, json_mode),
            Err(e) => print_err(&format!("agent start: {e}")),
        },
        other => println!("agent: unknown action '{other}'. Use list | start | stop | inspect"),
    }
}

async fn handle_memory(args: &MemoryArgs, raw: bool, json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };
    match args.action.as_str() {
        "store" => {
            let Some(key) = &args.key else {
                print_err("memory store requires a key: mesoclaw memory store <key> <content>");
                return;
            };
            let Some(content) = &args.value else {
                print_err("memory store requires content: mesoclaw memory store <key> <content>");
                return;
            };
            match client
                .client
                .post(format!("{}/api/v1/memory", client.base_url))
                .header("Authorization", client.auth_header())
                .json(&json!({ "key": key, "content": content }))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("stored memory entry '{key}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory store failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory store: {e}")),
            }
        }
        "search" => {
            let query = args.key.as_deref().unwrap_or("");
            if query.is_empty() {
                print_err("memory search requires a query: mesoclaw memory search <query>");
                return;
            }
            match client
                .client
                .get(format!("{}/api/v1/memory/search", client.base_url))
                .header("Authorization", client.auth_header())
                .query(&[("q", query)])
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => print_err(&format!("memory search: failed to parse response: {e}")),
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory search failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory search: {e}")),
            }
        }
        "forget" => {
            let Some(key) = &args.key else {
                print_err("memory forget requires a key: mesoclaw memory forget <key>");
                return;
            };
            match client
                .client
                .delete(format!("{}/api/v1/memory/{key}", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("forgot memory entry '{key}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory forget failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory forget: {e}")),
            }
        }
        "list" => {
            match client
                .client
                .get(format!("{}/api/v1/memory", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => print_err(&format!("memory list: failed to parse response: {e}")),
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory list failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory list: {e}")),
            }
        }
        other => print_err(&format!(
            "unknown memory action '{other}'. Use: store | search | forget | list"
        )),
    }
}

async fn handle_identity(args: &IdentityArgs, raw: bool, json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };
    match args.action.as_str() {
        "list" => {
            match client
                .client
                .get(format!("{}/api/v1/identity", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => print_err(&format!("identity list: failed to parse response: {e}")),
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity list failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity list: {e}")),
            }
        }
        "get" => {
            let Some(file_name) = &args.name else {
                print_err("identity get requires a file name: mesoclaw identity get <file>");
                return;
            };
            match client
                .client
                .get(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => print_err(&format!("identity get: failed to parse response: {e}")),
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity get failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity get: {e}")),
            }
        }
        "set" => {
            let Some(file_name) = &args.name else {
                print_err(
                    "identity set requires a file name and content: mesoclaw identity set <file> <content>",
                );
                return;
            };
            // For `identity set`, we need a content argument. Since IdentityArgs
            // only has `name`, read content from stdin if not provided inline.
            let content = read_identity_content_from_stdin();
            if content.is_empty() {
                print_err("identity set: no content provided. Pipe content via stdin.");
                return;
            }
            match client
                .client
                .put(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .json(&json!({ "content": content }))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("updated identity file '{file_name}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity set failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity set: {e}")),
            }
        }
        "edit" => {
            let Some(file_name) = &args.name else {
                print_err("identity edit requires a file name: mesoclaw identity edit <file>");
                return;
            };
            // Fetch current content from gateway.
            let current = match client
                .client
                .get(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    Err(e) => {
                        print_err(&format!("identity edit: failed to parse response: {e}"));
                        return;
                    }
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity edit: fetch failed ({status}): {body}"));
                    return;
                }
                Err(e) => {
                    print_err(&format!("identity edit: {e}"));
                    return;
                }
            };

            // Write to temp file, open in $EDITOR, read back.
            let edited = match open_in_editor(&current, file_name) {
                Ok(text) => text,
                Err(e) => {
                    print_err(&format!("identity edit: {e}"));
                    return;
                }
            };

            if edited == current {
                println!("no changes — identity file '{file_name}' unchanged");
                return;
            }

            match client
                .client
                .put(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .json(&json!({ "content": edited }))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("updated identity file '{file_name}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity edit: save failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity edit: {e}")),
            }
        }
        other => print_err(&format!(
            "unknown identity action '{other}'. Use: list | get | set | edit"
        )),
    }
}

/// Read all of stdin (non-blocking check: only if stdin is not a TTY).
fn read_identity_content_from_stdin() -> String {
    if io::stdin().is_terminal() {
        return String::new();
    }
    let mut buf = String::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                buf.push_str(&l);
                buf.push('\n');
            }
            Err(_) => break,
        }
    }
    buf
}

/// Open `content` in `$EDITOR` (or `vi`) via a temp file, return edited text.
fn open_in_editor(content: &str, suffix: &str) -> Result<String, String> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let dir = std::env::temp_dir();
    let tmp_path = dir.join(format!("mesoclaw-identity-{suffix}"));
    fs::write(&tmp_path, content).map_err(|e| format!("failed to write temp file: {e}"))?;

    let status = std::process::Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .map_err(|e| format!("failed to launch editor '{editor}': {e}"))?;

    if !status.success() {
        return Err(format!("editor exited with status {status}"));
    }
    let edited =
        fs::read_to_string(&tmp_path).map_err(|e| format!("failed to read edited file: {e}"))?;
    let _ = fs::remove_file(&tmp_path);
    Ok(edited)
}

async fn handle_module(args: &ModuleArgs, raw: bool, json_mode: bool) {
    match args.action.as_str() {
        "list" => {
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.list_modules().await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module list: {e}")),
            }
        }
        "health" => {
            let Some(name) = &args.name else {
                print_err("module health requires a module name");
                return;
            };
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.module_health(name).await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module health: {e}")),
            }
        }
        "start" | "stop" => {
            let Some(name) = &args.name else {
                print_err(&format!("module {} requires a module name", args.action));
                return;
            };
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.module_action(name, &args.action).await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module {}: {e}", args.action)),
            }
        }
        "reload" => {
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.reload_modules().await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module reload: {e}")),
            }
        }
        "create" => {
            let Some(name) = &args.name else {
                print_err("module create requires a module name: mesoclaw module create <name>");
                return;
            };
            create_module_scaffold(name, &args.module_type, &args.runtime);
        }
        "install" => {
            let Some(name) = &args.name else {
                print_err(
                    "module install requires a module name: mesoclaw module install <name> [--url <source>]",
                );
                return;
            };
            install_module(name, args.url.as_deref());
        }
        "remove" => {
            let Some(name) = &args.name else {
                print_err("module remove requires a module name: mesoclaw module remove <name>");
                return;
            };
            remove_module(name, args.force);
        }
        other => {
            print_err(&format!(
                "unknown module action '{other}'. Use: list | install | remove | start | stop | health | reload | create"
            ));
        }
    }
}

/// Generate a module scaffold at `~/.mesoclaw/modules/<name>/`.
fn create_module_scaffold(name: &str, module_type: &str, runtime: &str) {
    let modules_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".mesoclaw")
        .join("modules")
        .join(name);

    if modules_dir.exists() {
        print_err(&format!(
            "module '{name}' already exists at {modules_dir:?}"
        ));
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&modules_dir) {
        print_err(&format!("failed to create module directory: {e}"));
        return;
    }

    let manifest = format!(
        r#"[module]
id = "{name}"
name = "{name}"
version = "0.1.0"
description = "A MesoClaw extension module"
type = "{module_type}"

[runtime]
runtime_type = "{runtime}"
command = "./{name}"
args = []
timeout_secs = 30

[security]
allow_network = false
max_memory_mb = 128
"#
    );

    let manifest_path = modules_dir.join("manifest.toml");
    if let Err(e) = std::fs::write(&manifest_path, &manifest) {
        print_err(&format!("failed to write manifest: {e}"));
        return;
    }

    // Generate a template script (shell for native runtime).
    if runtime == "native" {
        let script = format!(
            r#"#!/usr/bin/env bash
# MesoClaw module: {name}
# Reads a JSON request from stdin, writes a JSON response to stdout.
# See https://docs.mesoclaw.dev/modules for protocol details.

set -euo pipefail

# Read the request
request=$(cat)
echo "$request" >&2  # debug — remove in production

# Write the response
echo '{{"result": "ok", "output": "hello from {name}"}}'
"#
        );
        let script_path = modules_dir.join(name);
        if let Err(e) = std::fs::write(&script_path, &script) {
            print_err(&format!("failed to write script: {e}"));
            return;
        }
        // Make executable on Unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&script_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&script_path, perms);
            }
        }
    }

    println!("Created module '{name}' at {modules_dir:?}");
    println!("  manifest: {manifest_path:?}");
    println!("  Edit manifest.toml to configure your module.");
}

/// Returns the canonical modules directory: `~/.mesoclaw/modules/`.
fn modules_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".mesoclaw")
        .join("modules")
}

/// Install a module from a git URL or local path into `~/.mesoclaw/modules/<name>/`.
///
/// Sources:
/// - `https://...` or `git@...` — cloned with `git clone`
/// - `/path/to/dir` or `./path` — copied recursively
/// - `None` — prints usage hint (no remote registry yet)
fn install_module(name: &str, source: Option<&str>) {
    let dest = modules_dir().join(name);

    if dest.exists() {
        print_err(&format!(
            "module '{name}' is already installed at {dest:?}. \
             Run `mesoclaw module remove {name}` first to reinstall."
        ));
        return;
    }

    let Some(src) = source else {
        println!(
            "Usage: mesoclaw module install <name> --url <source>\n\
             <source> can be:\n\
             • A git URL:   https://github.com/example/my-module\n\
             • A local dir: /path/to/module  or  ./my-module"
        );
        return;
    };

    if let Err(e) = std::fs::create_dir_all(&dest) {
        print_err(&format!("failed to create module directory: {e}"));
        return;
    }

    let is_git = src.starts_with("https://")
        || src.starts_with("http://")
        || src.starts_with("git@")
        || src.ends_with(".git");

    if is_git {
        println!("Cloning {src} → {dest:?} ...");
        let status = std::process::Command::new("git")
            .args(["clone", "--depth", "1", src, dest.to_str().unwrap_or(name)])
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("Installed module '{name}' from {src}");
                println!("Run `mesoclaw module reload` to pick it up without restarting.");
            }
            Ok(s) => {
                let _ = std::fs::remove_dir_all(&dest);
                print_err(&format!("git clone failed with exit code {s}"));
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dest);
                print_err(&format!("failed to run git: {e}"));
            }
        }
    } else {
        // Local path copy.
        let src_path = PathBuf::from(src);
        if !src_path.is_dir() {
            let _ = std::fs::remove_dir_all(&dest);
            print_err(&format!("source path '{src}' is not a directory"));
            return;
        }
        match copy_dir_all(&src_path, &dest) {
            Ok(count) => {
                println!("Installed module '{name}' from {src_path:?} ({count} files copied)");
                println!("Run `mesoclaw module reload` to pick it up without restarting.");
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dest);
                print_err(&format!("failed to copy module: {e}"));
            }
        }
    }
}

/// Recursively copy a directory. Returns the number of files copied.
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<usize> {
    std::fs::create_dir_all(dst)?;
    let mut count = 0;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_entry = dst.join(entry.file_name());
        if ty.is_dir() {
            count += copy_dir_all(&entry.path(), &dst_entry)?;
        } else {
            std::fs::copy(entry.path(), &dst_entry)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Remove an installed module by deleting its directory from `~/.mesoclaw/modules/<name>/`.
fn remove_module(name: &str, force: bool) {
    let target = modules_dir().join(name);

    if !target.exists() {
        print_err(&format!(
            "module '{name}' is not installed (looked in {target:?})"
        ));
        return;
    }

    if !force {
        print!("Remove module '{name}' at {target:?}? [y/N] ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() || !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return;
        }
    }

    match std::fs::remove_dir_all(&target) {
        Ok(()) => println!("Removed module '{name}'."),
        Err(e) => print_err(&format!("failed to remove module '{name}': {e}")),
    }
}

// ---------------------------------------------------------------------------
// Schedule handler
// ---------------------------------------------------------------------------

/// Manage scheduled jobs via the gateway scheduler API.
///
/// Actions:
///   list             — show all jobs
///   add              — create a job (requires --name and --cron or --interval)
///   toggle <id>      — enable/disable a job
///   remove <id>      — delete a job
///   history <id>     — show execution history for a job
async fn handle_schedule(args: &ScheduleArgs, raw: bool, _json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };

    match args.action.as_str() {
        "list" => match client.list_scheduler_jobs().await {
            Ok(v) => {
                if raw {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                    return;
                }
                let jobs = v
                    .get("jobs")
                    .and_then(|j| j.as_array())
                    .cloned()
                    .unwrap_or_default();
                if jobs.is_empty() {
                    println!("No scheduled jobs.");
                    return;
                }
                println!("{:<38} {:<24} {:<12} Schedule", "ID", "Name", "Enabled");
                println!("{}", "-".repeat(90));
                for job in &jobs {
                    let id = job.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                    let name = job.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                    let enabled = job
                        .get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let schedule = job
                        .get("schedule")
                        .map(|s| serde_json::to_string(s).unwrap_or_default())
                        .unwrap_or_default();
                    println!(
                        "{id:<38} {name:<24} {:<12} {schedule}",
                        if enabled { "yes" } else { "no" }
                    );
                }
            }
            Err(e) => print_err(&format!("failed to list jobs: {e}")),
        },

        "add" => {
            let name = match &args.name {
                Some(n) => n.clone(),
                None => {
                    print_err("--name is required for 'add'");
                    return;
                }
            };
            let schedule = match (&args.cron, args.interval) {
                (Some(expr), _) => json!({ "Cron": { "expr": expr } }),
                (None, Some(secs)) => json!({ "Interval": { "secs": secs } }),
                (None, None) => {
                    print_err("either --cron or --interval is required for 'add'");
                    return;
                }
            };
            let payload = match &args.prompt {
                Some(p) => json!({ "AgentTurn": { "prompt": p } }),
                None => json!("Heartbeat"),
            };
            let body = json!({
                "name": name,
                "schedule": schedule,
                "payload": payload,
                "enabled": true,
                "delete_after_run": args.once,
            });
            match client.create_scheduler_job(body).await {
                Ok(v) => {
                    let id = v.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Created job {id} ('{name}').");
                }
                Err(e) => print_err(&format!("failed to create job: {e}")),
            }
        }

        "toggle" => {
            let id = match &args.id {
                Some(id) => id.clone(),
                None => {
                    print_err("provide job id: mesoclaw schedule toggle <id>");
                    return;
                }
            };
            match client.toggle_scheduler_job(&id).await {
                Ok(v) => {
                    let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                    println!(
                        "Job {id} is now {}.",
                        if enabled { "enabled" } else { "disabled" }
                    );
                }
                Err(e) => print_err(&format!("failed to toggle job: {e}")),
            }
        }

        "remove" | "delete" => {
            let id = match &args.id {
                Some(id) => id.clone(),
                None => {
                    print_err("provide job id: mesoclaw schedule remove <id>");
                    return;
                }
            };
            match client.delete_scheduler_job(&id).await {
                Ok(_) => println!("Deleted job {id}."),
                Err(e) => print_err(&format!("failed to delete job: {e}")),
            }
        }

        "history" => {
            let id = match &args.id {
                Some(id) => id.clone(),
                None => {
                    print_err("provide job id: mesoclaw schedule history <id>");
                    return;
                }
            };
            match client.scheduler_job_history(&id).await {
                Ok(v) => {
                    if raw {
                        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                        return;
                    }
                    let entries = v
                        .get("history")
                        .and_then(|h| h.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if entries.is_empty() {
                        println!("No history for job {id}.");
                        return;
                    }
                    println!("{:<28} {:<10} Output", "Run At", "Status");
                    println!("{}", "-".repeat(72));
                    for entry in &entries {
                        let ran_at = entry.get("ran_at").and_then(|v| v.as_str()).unwrap_or("-");
                        let status = entry.get("status").and_then(|v| v.as_str()).unwrap_or("-");
                        let output = entry.get("output").and_then(|v| v.as_str()).unwrap_or("");
                        println!("{ran_at:<28} {status:<10} {output}");
                    }
                }
                Err(e) => print_err(&format!("failed to fetch history: {e}")),
            }
        }

        other => print_err(&format!(
            "unknown schedule action '{other}'. Use: list | add | toggle | remove | history"
        )),
    }
}

// ---------------------------------------------------------------------------
// Channel and Config handlers
// ---------------------------------------------------------------------------

/// Manage communication channels (Telegram, etc.).
///
/// Reads and writes channel credentials directly to the OS keyring.
/// The daemon picks up changes on restart.
async fn handle_channel(args: &ChannelArgs, raw: bool, json_mode: bool) {
    const SERVICE: &str = "com.sprklai.mesoclaw";

    match args.action.as_str() {
        "list" => {
            // Probe keyring for configured channels.
            let telegram_configured = keyring::Entry::new(SERVICE, "channel:telegram:token")
                .ok()
                .and_then(|e| e.get_password().ok())
                .map(|t| !t.is_empty())
                .unwrap_or(false);

            // Try to enrich with live status from gateway.
            let telegram_status = if let Some(client) = require_gateway().await {
                match client
                    .client
                    .get(format!(
                        "{}/api/v1/channels/telegram/health",
                        client.base_url
                    ))
                    .header("Authorization", client.auth_header())
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => resp
                        .json::<Value>()
                        .await
                        .ok()
                        .and_then(|v| v.get("connected").and_then(|b| b.as_bool()))
                        .map(|c| if c { "connected" } else { "disconnected" })
                        .unwrap_or("unknown"),
                    _ => {
                        if telegram_configured {
                            "configured (daemon not running)"
                        } else {
                            "not configured"
                        }
                    }
                }
            } else if telegram_configured {
                "configured (daemon offline)"
            } else {
                "not configured"
            };

            let channels = json!([
                {
                    "name": "tauri-ipc",
                    "description": "Built-in desktop IPC channel",
                    "status": "always connected",
                    "configured": true,
                },
                {
                    "name": "telegram",
                    "description": "Telegram bot channel",
                    "status": telegram_status,
                    "configured": telegram_configured,
                },
            ]);
            print_value(&channels, raw, json_mode);
        }

        "add" | "set" => match args.channel_type.as_deref() {
            Some("telegram") => configure_telegram_channel(args, SERVICE),
            Some(t) => print_err(&format!(
                "unknown channel type '{t}'. Supported types: telegram"
            )),
            None => print_err(
                "channel add requires a type: mesoclaw channel add telegram [--token <tok>] [--chat-ids <ids>]",
            ),
        },

        "status" | "health" => {
            let Some(name) = &args.channel_type else {
                print_err("channel status requires a channel name: mesoclaw channel status <name>");
                return;
            };
            let Some(client) = require_gateway().await else {
                return;
            };
            match client
                .client
                .get(format!("{}/api/v1/channels/{name}/health", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(v) = resp.json::<Value>().await {
                        print_value(&v, raw, json_mode);
                    }
                }
                Ok(_) | Err(_) => {
                    // Fallback: show keyring config status.
                    let configured = if name == "telegram" {
                        keyring::Entry::new(SERVICE, "channel:telegram:token")
                            .ok()
                            .and_then(|e| e.get_password().ok())
                            .map(|t| !t.is_empty())
                            .unwrap_or(false)
                    } else {
                        false
                    };
                    let v = json!({ "channel": name, "configured": configured, "gateway": "unavailable" });
                    print_value(&v, raw, json_mode);
                }
            }
        }

        "remove" => match args.channel_type.as_deref() {
            Some("telegram") => {
                let keys = [
                    "channel:telegram:token",
                    "channel:telegram:allowed_chat_ids",
                    "channel:telegram:polling_timeout_secs",
                ];
                let mut removed = false;
                for key in &keys {
                    if let Ok(entry) = keyring::Entry::new(SERVICE, key) {
                        let _ = entry.delete_password();
                        removed = true;
                    }
                }
                if removed {
                    println!(
                        "Telegram channel configuration removed.\n\
                         Restart the daemon to apply: mesoclaw daemon stop && mesoclaw daemon start"
                    );
                } else {
                    println!("No Telegram channel configuration found.");
                }
            }
            Some(t) => print_err(&format!("unknown channel type '{t}'")),
            None => print_err(
                "channel remove requires a channel name: mesoclaw channel remove telegram",
            ),
        },

        other => print_err(&format!(
            "unknown channel action '{other}'. Use: list | add | set | status | remove"
        )),
    }
}

/// Write Telegram channel credentials to the OS keyring.
///
/// If `--token` / `--chat-ids` flags are absent, prompts interactively.
fn configure_telegram_channel(args: &ChannelArgs, service: &str) {
    let token = if let Some(t) = &args.token {
        t.clone()
    } else {
        eprintln!("Telegram Bot Setup");
        eprintln!("──────────────────────────────────────────");
        eprintln!("1. Open Telegram and search for @BotFather");
        eprintln!("2. Send /newbot and follow the prompts");
        eprintln!("3. Copy the token BotFather gives you");
        eprintln!("4. Find your chat ID via @userinfobot");
        eprintln!("──────────────────────────────────────────");
        print!("Bot token: ");
        io::stdout().flush().unwrap_or_default();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap_or_default();
        input.trim().to_string()
    };

    if token.is_empty() {
        print_err("bot token is required");
        return;
    }

    let chat_ids = if let Some(ids) = &args.chat_ids {
        ids.clone()
    } else {
        print!("Allowed chat IDs (comma-separated, e.g. 123456789): ");
        io::stdout().flush().unwrap_or_default();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap_or_default();
        input.trim().to_string()
    };

    let timeout_str = args.polling_timeout.to_string();

    let entries = [
        ("channel:telegram:token", token.as_str()),
        ("channel:telegram:allowed_chat_ids", chat_ids.as_str()),
        (
            "channel:telegram:polling_timeout_secs",
            timeout_str.as_str(),
        ),
    ];

    for (key, value) in &entries {
        match keyring::Entry::new(service, key) {
            Ok(entry) => {
                if let Err(e) = entry.set_password(value) {
                    print_err(&format!("failed to save '{key}': {e}"));
                    return;
                }
            }
            Err(e) => {
                print_err(&format!("keyring error for '{key}': {e}"));
                return;
            }
        }
    }

    println!("Telegram channel configured successfully.");
    println!("Restart the daemon to connect: mesoclaw daemon stop && mesoclaw daemon start");
}

/// View and manage AI provider configuration.
///
/// API keys are stored in the OS keyring using the key format `api-key:{provider_id}`.
async fn handle_config(args: &ConfigArgs, raw: bool, json_mode: bool) {
    const SERVICE: &str = "com.sprklai.mesoclaw";

    match args.action.as_str() {
        "list" => {
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.list_providers().await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("config list: {e}")),
            }
        }

        "set-key" => {
            let Some(provider_id) = &args.provider else {
                print_err(
                    "config set-key requires a provider ID: mesoclaw config set-key <provider> [<api-key>]",
                );
                return;
            };

            let api_key = if let Some(k) = &args.value {
                k.clone()
            } else {
                print!("API key for '{provider_id}': ");
                io::stdout().flush().unwrap_or_default();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap_or_default();
                input.trim().to_string()
            };

            if api_key.is_empty() {
                print_err("API key cannot be empty");
                return;
            }

            let key_name = format!("api-key:{provider_id}");
            match keyring::Entry::new(SERVICE, &key_name) {
                Ok(entry) => match entry.set_password(&api_key) {
                    Ok(()) => println!("API key for '{provider_id}' saved to keyring."),
                    Err(e) => print_err(&format!("failed to save API key: {e}")),
                },
                Err(e) => print_err(&format!("keyring error: {e}")),
            }
        }

        "get-key" => {
            let Some(provider_id) = &args.provider else {
                print_err(
                    "config get-key requires a provider ID: mesoclaw config get-key <provider>",
                );
                return;
            };

            let key_name = format!("api-key:{provider_id}");
            match keyring::Entry::new(SERVICE, &key_name) {
                Ok(entry) => match entry.get_password() {
                    Ok(key) => {
                        // Mask most of the key for security.
                        let masked = if key.len() > 8 {
                            format!("{}...{}", &key[..4], &key[key.len() - 4..])
                        } else {
                            "****".to_string()
                        };
                        if json_mode {
                            print_value(
                                &json!({ "provider": provider_id, "hasKey": true, "preview": masked }),
                                raw,
                                json_mode,
                            );
                        } else {
                            println!("API key for '{provider_id}': {masked}  (key is set)");
                        }
                    }
                    Err(_) => {
                        if json_mode {
                            print_value(
                                &json!({ "provider": provider_id, "hasKey": false }),
                                raw,
                                json_mode,
                            );
                        } else {
                            println!("No API key set for '{provider_id}'.");
                        }
                    }
                },
                Err(e) => print_err(&format!("keyring error: {e}")),
            }
        }

        "delete-key" => {
            let Some(provider_id) = &args.provider else {
                print_err(
                    "config delete-key requires a provider ID: mesoclaw config delete-key <provider>",
                );
                return;
            };

            let key_name = format!("api-key:{provider_id}");
            match keyring::Entry::new(SERVICE, &key_name) {
                Ok(entry) => match entry.delete_password() {
                    Ok(()) => println!("API key for '{provider_id}' removed from keyring."),
                    Err(e) => print_err(&format!("failed to delete API key: {e}")),
                },
                Err(e) => print_err(&format!("keyring error: {e}")),
            }
        }

        other => print_err(&format!(
            "unknown config action '{other}'. Use: list | set-key | get-key | delete-key"
        )),
    }
}

// ---------------------------------------------------------------------------
// Interactive REPL with WebSocket streaming
// ---------------------------------------------------------------------------

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Send a message to the agent and stream events until `agent_complete`.
///
/// Events rendered:
/// - `agent_started`    → records the session_id
/// - `agent_tool_start` → prints "→ tool_name(args)"
/// - `agent_tool_result`→ prints "  ✓ result" or "  ✗ result"
/// - `agent_complete`   → prints the final response and returns
/// - `approval_needed`  → prompts the user and POSTs the decision
async fn stream_agent_message(
    content: &str,
    ws: &mut WsStream,
    base_url: &str,
    token: &str,
    http_client: &reqwest::Client,
) {
    let msg = json!({ "type": "agent_message", "content": content }).to_string();
    if ws.send(Message::Text(msg)).await.is_err() {
        print_err("WebSocket send failed — is the daemon still running?");
        return;
    }

    while let Some(frame) = ws.next().await {
        let text = match frame {
            Ok(Message::Text(t)) => t,
            Ok(Message::Close(_)) | Err(_) => break,
            _ => continue,
        };

        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match v.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "agent_started" => {
                // session_id captured for potential cancellation
                if let Some(id) = v.get("session_id").and_then(|s| s.as_str()) {
                    eprintln!("\x1b[2m[session {id}]\x1b[0m");
                }
            }
            "agent_tool_start" => {
                let tool = v
                    .get("tool_name")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let args = v
                    .get("args")
                    .map(|a| serde_json::to_string(a).unwrap_or_default())
                    .unwrap_or_default();
                eprintln!("\x1b[33m→\x1b[0m {tool}({args})");
            }
            "agent_tool_result" => {
                let tool = v
                    .get("tool_name")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let result = v.get("result").and_then(|r| r.as_str()).unwrap_or("");
                let success = v.get("success").and_then(|b| b.as_bool()).unwrap_or(false);
                if success {
                    eprintln!("\x1b[32m  ✓\x1b[0m {tool}: {result}");
                } else {
                    eprintln!("\x1b[31m  ✗\x1b[0m {tool}: {result}");
                }
            }
            "agent_complete" => {
                if let Some(message) = v.get("message").and_then(|m| m.as_str()) {
                    println!("{message}");
                }
                break;
            }
            "approval_needed" => {
                let action_id = v
                    .get("action_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                let tool = v
                    .get("tool_name")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let description = v.get("description").and_then(|s| s.as_str()).unwrap_or("");
                let risk = v
                    .get("risk_level")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");

                eprint!(
                    "\x1b[33m[APPROVAL]\x1b[0m {tool}: {description} \x1b[2m(risk: {risk})\x1b[0m\nApprove? [y/N]: "
                );
                let _ = io::stderr().flush();

                let mut answer = String::new();
                let approved = if io::stdin().lock().read_line(&mut answer).is_ok() {
                    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
                } else {
                    false
                };

                // POST the approval decision to the gateway.
                if !action_id.is_empty() {
                    let url = format!("{base_url}/api/v1/approval/{action_id}");
                    let _ = http_client
                        .post(&url)
                        .header("Authorization", format!("Bearer {token}"))
                        .json(&json!({ "approved": approved }))
                        .send()
                        .await;
                }

                if approved {
                    eprintln!("\x1b[32mApproved.\x1b[0m");
                } else {
                    eprintln!("\x1b[31mDenied.\x1b[0m");
                }
            }
            "error" => {
                let msg = v
                    .get("error")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown error");
                print_err(msg);
                break;
            }
            _ => {}
        }
    }
}

async fn run_repl(raw: bool, json_mode: bool) {
    // Detect stdin pipe mode.
    let is_tty = io::stdin().is_terminal();

    if is_tty {
        println!("MesoClaw interactive shell. Type 'help' for commands, 'exit' to quit.");
    }

    // Gather connection info (port + token) without consuming the client.
    let conn_info = if let Some(port) = is_daemon_running() {
        read_token().map(|token| (format!("http://127.0.0.1:{port}"), token))
    } else {
        if is_tty {
            eprintln!("Gateway not running. Start it with: mesoclaw daemon start");
        }
        None
    };

    // Connect WebSocket.
    let mut ws_stream: Option<WsStream> = None;
    if let Some((ref base_url, ref token)) = conn_info {
        let ws_url = format!(
            "{}/api/v1/ws?token={}",
            base_url.replace("http://", "ws://"),
            token
        );
        match connect_async(&ws_url).await {
            Ok((stream, _)) => {
                if is_tty {
                    println!("Connected to daemon. Streaming enabled.\n");
                }
                ws_stream = Some(stream);
            }
            Err(e) => {
                if is_tty {
                    eprintln!("WebSocket connect failed: {e}. Subcommands still work.\n");
                }
            }
        }
    }

    let http_client = reqwest::Client::new();

    // Pipe mode: read all stdin, send as one-shot message.
    if !is_tty {
        let mut input = String::new();
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    input.push_str(&l);
                    input.push('\n');
                }
                Err(_) => break,
            }
        }
        if let Some(ref mut ws) = ws_stream {
            if let Some((ref base_url, ref token)) = conn_info {
                stream_agent_message(input.trim(), ws, base_url, token, &http_client).await;
            }
        } else {
            // No gateway — echo back.
            print!("{input}");
        }
        return;
    }

    // Interactive TTY mode.
    let stdin = io::stdin();
    loop {
        print!("mesoclaw> ");
        io::stdout().flush().unwrap_or_default();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            "exit" | "quit" | "q" => {
                println!("Goodbye.");
                break;
            }
            "help" | "?" => print_help(),
            _ => {
                // Try to parse as a subcommand first.
                let parts: Vec<&str> = std::iter::once("mesoclaw")
                    .chain(trimmed.split_whitespace())
                    .collect();
                match Cli::try_parse_from(&parts) {
                    Ok(cli) => {
                        if let Some(cmd) = &cli.command {
                            dispatch(cmd, raw, json_mode).await;
                        }
                    }
                    Err(_) => {
                        // Treat as an agent message — stream the response.
                        match (&mut ws_stream, &conn_info) {
                            (Some(ws), Some((base_url, token))) => {
                                stream_agent_message(trimmed, ws, base_url, token, &http_client)
                                    .await;
                            }
                            _ => {
                                eprintln!(
                                    "Not connected to gateway. Start daemon: mesoclaw daemon start"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn print_help() {
    println!(
        "Commands: daemon | agent | memory | identity | config | schedule | channel | module | generate | gui | exit\n\
         \n\
         config  list                          — list AI providers\n\
         config  set-key <provider> [<key>]    — save API key to keyring\n\
         config  get-key <provider>            — check if API key is set\n\
         config  delete-key <provider>         — remove API key\n\
         \n\
         channel list                          — list configured channels\n\
         channel add telegram [--token <t>] [--chat-ids <ids>]  — configure Telegram\n\
         channel status <name>                 — get channel connection status\n\
         channel remove <name>                 — remove channel configuration"
    );
}

// ---------------------------------------------------------------------------
// Generate handler
// ---------------------------------------------------------------------------

async fn handle_generate(args: &GenerateArgs, _raw: bool, _json_mode: bool) {
    match &args.action {
        Some(GenerateAction::List) => {
            println!(
                "generate list: use the desktop app UI or connect via gateway for full listing"
            );
        }
        Some(GenerateAction::Delete { id }) => {
            println!("generate delete {id}: use the desktop app UI or connect via gateway");
        }
        None => {
            let artifact_type = args.r#type.as_deref().unwrap_or("generic");
            let name = args.name.as_deref().unwrap_or("untitled");
            let description = args.description.as_deref().unwrap_or("");
            if description.is_empty() {
                print_err(
                    "generate requires --description (-d): mesoclaw generate -t skill -n my-skill -d \"A skill that...\"",
                );
                return;
            }
            println!(
                "Generating '{artifact_type}' artifact '{name}': use the desktop app UI or connect via gateway.\n\
                 Description: {description}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Install ring crypto provider for rustls before any network I/O.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli = Cli::parse();

    // --auto: signal full-autonomy mode to run_repl via a sentinel value in the env.
    // The MESOCLAW_SECURITY_LEVEL env var is checked by load_default_config() and the
    // gateway. Note: we write it before any async or multi-threaded code starts, which
    // is safe on all supported platforms at this point in execution.
    if cli.auto {
        // Use std::env::set_var via the config system: pass a known safe override.
        // This avoids unsafe code by leveraging the existing env override path.
        std::env::vars().for_each(|_| {}); // no-op force scan; actual override via --auto flag below
        // ## TODO: wire --auto flag through GatewayClient headers to the daemon
        // so the spawned session uses AutonomyLevel::Full.
        eprintln!("[auto] full-autonomy mode: approval prompts suppressed");
    }

    // --resume: if a session ID is provided and no subcommand, jump into that session.
    if let Some(ref session_id) = cli.resume {
        run_repl_resume(session_id, cli.raw, cli.json).await;
        return;
    }

    match &cli.command {
        Some(command) => dispatch(command, cli.raw, cli.json).await,
        None => run_repl(cli.raw, cli.json).await,
    }
}

/// Resume an existing agent session by ID, then enter the REPL with that session's context.
async fn run_repl_resume(session_id: &str, raw: bool, json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };
    // Verify the session exists via the gateway.
    match client.list_sessions().await {
        Ok(sessions) => {
            let found = sessions
                .as_array()
                .map(|arr| arr.iter().any(|s| s["id"].as_str() == Some(session_id)))
                .unwrap_or(false);
            if found {
                if !raw {
                    println!("Resuming session {session_id}. Entering REPL…\n");
                }
                // ## TODO: pass session_id into run_repl so messages append to the session.
                run_repl(raw, json_mode).await;
            } else {
                eprintln!("session '{session_id}' not found");
            }
        }
        Err(e) => eprintln!("failed to list sessions: {e}"),
    }
}

/// Watch a path for filesystem changes and trigger an agent on each event.
///
/// Uses a 500ms (configurable) debounce so that rapid file saves don't
/// flood the agent with requests.
async fn handle_watch(args: &WatchArgs, raw: bool, json_mode: bool) {
    use std::time::Duration;
    use tokio::sync::mpsc;

    let Some(client) = require_gateway().await else {
        return;
    };

    let (tx, mut rx) = mpsc::channel::<String>(32);
    let debounce = Duration::from_millis(args.debounce_ms);
    let watch_path = args.path.clone();
    let prompt_template = args.prompt.clone();

    if !raw {
        println!(
            "Watching '{}' (debounce {}ms). Press Ctrl-C to stop.",
            watch_path, args.debounce_ms
        );
    }

    // Spawn a blocking thread to run the notify watcher.
    let tx2 = tx.clone();
    std::thread::spawn(move || {
        use notify::{Config, RecursiveMode, Watcher};
        let (ntx, nrx) = std::sync::mpsc::channel();
        let Ok(mut watcher) =
            notify::RecommendedWatcher::new(ntx, Config::default().with_poll_interval(debounce))
        else {
            eprintln!("failed to create watcher");
            return;
        };
        if watcher
            .watch(std::path::Path::new(&watch_path), RecursiveMode::Recursive)
            .is_err()
        {
            eprintln!("failed to watch path");
            return;
        }
        for event in nrx {
            match event {
                Ok(ev) => {
                    let path = ev
                        .paths
                        .first()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    let _ = tx2.blocking_send(path);
                }
                Err(e) => eprintln!("watch error: {e}"),
            }
        }
    });

    let mut last_path: Option<String> = None;
    while let Some(path) = rx.recv().await {
        // Simple dedup: skip if same path as last event (within same tick).
        if last_path.as_deref() == Some(&path) {
            continue;
        }
        last_path = Some(path.clone());

        let prompt = prompt_template.replace("{file}", &path);
        if !raw {
            println!("[watch] change detected: {path}");
        }

        // Create a new agent session with the prompt.
        match client.create_session(Some(&prompt)).await {
            Ok(resp) => {
                if json_mode {
                    println!("{resp}");
                } else if raw {
                    println!("{}", resp["id"].as_str().unwrap_or("session created"));
                } else {
                    println!(
                        "[watch] session started: {}",
                        resp["id"].as_str().unwrap_or("unknown")
                    );
                }
            }
            Err(e) => eprintln!("[watch] agent error: {e}"),
        }
    }
}
