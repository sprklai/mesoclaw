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
use tokio_tungstenite::{connect_async, tungstenite::Message};

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
    /// Launch the MesoClaw desktop GUI.
    Gui,
}

#[derive(Parser, Debug)]
struct DaemonArgs {
    /// Daemon action: start | stop | status | restart.
    #[arg(default_value = "status")]
    action: String,
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
    #[arg(default_value = "list")]
    action: String,
    key: Option<String>,
    value: Option<String>,
}

#[derive(Parser, Debug)]
struct ScheduleArgs {
    #[arg(default_value = "list")]
    action: String,
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct ChannelArgs {
    #[arg(default_value = "list")]
    action: String,
    channel_type: Option<String>,
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
    let path = dirs::home_dir()?
        .join(".mesoclaw")
        .join("daemon.token");
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

    fn ws_url(&self) -> String {
        self.base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://")
            + "/api/v1/ws"
    }
}

/// Resolve or start the gateway, returning a ready client.
async fn require_gateway() -> Option<GatewayClient> {
    if let Some(port) = is_daemon_running() {
        if let Some(token) = read_token() {
            return Some(GatewayClient::new(port, token));
        }
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
        println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
    } else if raw {
        if let Some(s) = value.as_str() {
            println!("{s}");
        } else {
            println!("{value}");
        }
    } else {
        // Human-friendly default.
        println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
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
        Commands::Config(args) => {
            println!("config {}: not yet implemented (Phase 5)", args.action);
        }
        Commands::Schedule(args) => {
            println!("schedule {}: not yet implemented (Phase 4)", args.action);
        }
        Commands::Channel(args) => {
            println!("channel {}: not yet implemented (Phase 7)", args.action);
        }
        Commands::Gui => {
            println!("gui: not yet implemented — launch mesoclaw-desktop directly");
        }
    }
}

async fn handle_daemon(args: &DaemonArgs) {
    match args.action.as_str() {
        "status" => {
            match is_daemon_running() {
                Some(port) => {
                    if let Some(client) = require_gateway().await {
                        match client.health().await {
                            Ok(v) => println!("daemon: running on port {port} — {v}"),
                            Err(e) => println!("daemon: port {port} (health check failed: {e})"),
                        }
                    }
                }
                None => println!("daemon: not running"),
            }
        }
        "start" => {
            if let Some(port) = is_daemon_running() {
                println!("daemon: already running on port {port}");
                return;
            }
            #[cfg(feature = "gateway")]
            {
                use std::sync::Arc;
                use local_ts_lib::{event_bus::TokioBroadcastBus, gateway::start_gateway};
                let bus = Arc::new(TokioBroadcastBus::new());
                println!("Starting daemon…");
                if let Err(e) = start_gateway(bus).await {
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

async fn handle_memory(args: &MemoryArgs, _raw: bool, _json_mode: bool) {
    // ## TODO: implement memory commands once memory service exists (Phase 3)
    println!("memory {}: not yet implemented (Phase 3)", args.action);
}

async fn handle_identity(args: &IdentityArgs, _raw: bool, _json_mode: bool) {
    // ## TODO: wire to identity REST endpoint (Phase 2.6 completion)
    println!("identity {}: not yet implemented (Phase 3)", args.action);
}

// ---------------------------------------------------------------------------
// Interactive REPL with WebSocket streaming
// ---------------------------------------------------------------------------

async fn run_repl(raw: bool, json_mode: bool) {
    // Detect stdin pipe mode.
    let is_tty = io::stdin().is_terminal();

    if is_tty {
        println!("MesoClaw interactive shell. Type 'help' for commands, 'exit' to quit.");
    }

    // Try to connect to the gateway WS endpoint.
    let ws_stream = if let Some(client) = require_gateway().await {
        let url = client.ws_url();
        let ws_url = format!("{url}?token={}", client.token);
        match connect_async(&ws_url).await {
            Ok((stream, _)) => {
                if is_tty {
                    println!("Connected to daemon. Streaming enabled.\n");
                }
                Some(stream)
            }
            Err(e) => {
                if is_tty {
                    eprintln!("WebSocket connect failed: {e}. Running in stub mode.\n");
                }
                None
            }
        }
    } else {
        None
    };

    // Pipe mode: read all stdin, send as one-shot message.
    if !is_tty {
        let mut input = String::new();
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => { input.push_str(&l); input.push('\n'); }
                Err(_) => break,
            }
        }
        if let Some(mut ws) = ws_stream {
            let msg = json!({ "type": "message", "content": input.trim() }).to_string();
            let _ = ws.send(Message::Text(msg)).await;
            while let Some(Ok(Message::Text(response))) = ws.next().await {
                let v: Value = serde_json::from_str(&response).unwrap_or(Value::String(response));
                print_value(&v, raw, json_mode);
            }
        } else {
            // No gateway — just echo back.
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
            Err(e) => { eprintln!("read error: {e}"); break; }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        match trimmed {
            "exit" | "quit" | "q" => { println!("Goodbye."); break; }
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
                        // Treat as a message to the agent.
                        eprintln!("(gateway streaming not yet implemented — Phase 3)");
                    }
                }
            }
        }
    }
}

fn print_help() {
    println!(
        "Commands: daemon | agent | memory | identity | config | schedule | channel | gui | exit"
    );
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Some(command) => dispatch(command, cli.raw, cli.json).await,
        None => run_repl(cli.raw, cli.json).await,
    }
}
