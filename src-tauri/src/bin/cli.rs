/// MesoClaw CLI — headless interface to the AI agent runtime.
///
/// Provides subcommands for managing the daemon, agents, memory, identity,
/// configuration, scheduling, channels, and launching the GUI. When invoked
/// with no subcommand the CLI enters an interactive REPL.
///
/// # CI matrix note
/// The following feature combinations should be tested in CI:
///   - `cargo build --features core,cli`           (minimal: no desktop, no gateway)
///   - `cargo build`                                (default features)
///   - `cargo build --all-features`                (full build)
use clap::{Parser, Subcommand};
use std::io::{self, BufRead, Write};

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
    /// Subcommand to run. Omit for interactive REPL.
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
    /// Manage agent identities and credentials.
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

// ---------------------------------------------------------------------------
// Per-subcommand argument structs
// ---------------------------------------------------------------------------

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
    /// Optional agent name or ID to target.
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct MemoryArgs {
    /// Memory action: list | get | set | delete.
    #[arg(default_value = "list")]
    action: String,
    /// Memory key.
    key: Option<String>,
    /// Memory value (for set action).
    value: Option<String>,
}

#[derive(Parser, Debug)]
struct IdentityArgs {
    /// Identity action: list | create | delete | show.
    #[arg(default_value = "list")]
    action: String,
    /// Identity name or ID.
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct ConfigArgs {
    /// Config action: get | set | list | reset.
    #[arg(default_value = "list")]
    action: String,
    /// Configuration key.
    key: Option<String>,
    /// Configuration value (for set action).
    value: Option<String>,
}

#[derive(Parser, Debug)]
struct ScheduleArgs {
    /// Schedule action: list | add | remove | run.
    #[arg(default_value = "list")]
    action: String,
    /// Schedule name or ID.
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct ChannelArgs {
    /// Channel action: list | connect | disconnect | status.
    #[arg(default_value = "list")]
    action: String,
    /// Channel type (e.g., telegram).
    channel_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

fn dispatch(command: &Commands, _raw: bool, _json: bool) {
    match command {
        Commands::Daemon(args) => {
            // ## TODO: Implement daemon management once gateway exists (Phase 2).
            println!("daemon {}: not yet implemented", args.action);
        }
        Commands::Agent(args) => {
            // ## TODO: Implement agent management once gateway exists (Phase 2).
            println!("agent {}: not yet implemented", args.action);
        }
        Commands::Memory(args) => {
            // ## TODO: Implement memory management once gateway exists (Phase 2).
            println!("memory {}: not yet implemented", args.action);
        }
        Commands::Identity(args) => {
            // ## TODO: Implement identity management once gateway exists (Phase 2).
            println!("identity {}: not yet implemented", args.action);
        }
        Commands::Config(args) => {
            // ## TODO: Implement config management once gateway exists (Phase 2).
            println!("config {}: not yet implemented", args.action);
        }
        Commands::Schedule(args) => {
            // ## TODO: Implement schedule management once gateway exists (Phase 2).
            println!("schedule {}: not yet implemented", args.action);
        }
        Commands::Channel(args) => {
            // ## TODO: Implement channel management once gateway exists (Phase 2).
            println!("channel {}: not yet implemented", args.action);
        }
        Commands::Gui => {
            // ## TODO: Launch desktop binary process once inter-process communication is set up.
            println!("gui: not yet implemented");
        }
    }
}

// ---------------------------------------------------------------------------
// Interactive REPL
// ---------------------------------------------------------------------------

fn run_repl(raw: bool, json: bool) {
    println!("MesoClaw interactive shell. Type 'help' for commands, 'exit' to quit.");
    println!("Note: gateway not connected — most commands will not function until Phase 2.\n");

    let stdin = io::stdin();
    loop {
        print!("mesoclaw> ");
        io::stdout().flush().unwrap_or_else(|e| eprintln!("flush error: {e}"));

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {e}");
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
            "help" | "?" => {
                println!("Available subcommands:");
                println!("  daemon    — manage the background daemon");
                println!("  agent     — manage AI agents");
                println!("  memory    — manage memory stores");
                println!("  identity  — manage identities");
                println!("  config    — view/edit configuration");
                println!("  schedule  — manage scheduled tasks");
                println!("  channel   — manage communication channels");
                println!("  gui       — launch the desktop GUI");
                println!("  exit/quit — exit the REPL");
                println!();
                println!("Note: gateway not connected — commands are stubs until Phase 2.");
            }
            _ => {
                // Try to parse as a subcommand via clap
                let mut parts: Vec<&str> = std::iter::once("mesoclaw")
                    .chain(trimmed.split_whitespace())
                    .collect();
                // Append --raw/--json flags if they were passed to the REPL session
                let raw_flag = "--raw".to_string();
                let json_flag = "--json".to_string();
                if raw {
                    parts.push(&raw_flag);
                }
                if json {
                    parts.push(&json_flag);
                }
                match Cli::try_parse_from(&parts) {
                    Ok(cli) => {
                        if let Some(cmd) = &cli.command {
                            dispatch(cmd, raw, json);
                        }
                    }
                    Err(e) => {
                        // Print only the error message, not full clap help, for REPL UX
                        println!("Unknown command: '{trimmed}'. Type 'help' for commands.");
                        let _ = e; // suppress unused warning
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(command) => dispatch(command, cli.raw, cli.json),
        None => run_repl(cli.raw, cli.json),
    }
}
