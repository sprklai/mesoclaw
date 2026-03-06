mod client;
mod commands;

use std::process;

use clap::{Parser, Subcommand};

use client::MesoClient;

#[derive(Parser)]
#[command(
    name = "mesoclaw",
    about = "MesoClaw CLI — talk to your local AI agent"
)]
struct Cli {
    /// Daemon host address
    #[arg(long, default_value = "127.0.0.1", global = true)]
    host: String,

    /// Daemon port
    #[arg(long, default_value_t = 18981, global = true)]
    port: u16,

    /// Auth token (or set MESOCLAW_TOKEN env var)
    #[arg(long, global = true, env = "MESOCLAW_TOKEN")]
    token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage the daemon process
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Interactive chat with the AI agent (WebSocket)
    Chat {
        /// Session ID to continue a conversation
        #[arg(long)]
        session: Option<String>,
        /// Model override
        #[arg(long)]
        model: Option<String>,
    },
    /// Send a single prompt and print the response
    Run {
        /// The prompt to send
        prompt: String,
        /// Session ID
        #[arg(long)]
        session: Option<String>,
        /// Model override
        #[arg(long)]
        model: Option<String>,
    },
    /// Manage memory entries
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// View or update configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage API keys
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon process
    Start,
    /// Stop the daemon process
    Stop,
    /// Check daemon status
    Status,
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Search memories
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(long)]
        limit: Option<usize>,
        /// Offset for pagination
        #[arg(long)]
        offset: Option<usize>,
    },
    /// Add a memory entry
    Add {
        /// Memory key
        key: String,
        /// Memory content
        content: String,
    },
    /// Remove a memory entry
    Remove {
        /// Memory key to remove
        key: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    /// Set an API key for a provider
    Set {
        /// Provider name (e.g. openai, anthropic)
        provider: String,
        /// API key value
        key: String,
    },
    /// Remove an API key for a provider
    Remove {
        /// Provider name
        provider: String,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let client = MesoClient::new(&cli.host, cli.port, cli.token);

    let result = match cli.command {
        Commands::Daemon { action } => match action {
            DaemonAction::Start => commands::daemon::start().await,
            DaemonAction::Stop => commands::daemon::stop().await,
            DaemonAction::Status => commands::daemon::status(&client).await,
        },
        Commands::Chat { session, model } => {
            commands::chat::run(&client, session.as_deref(), model.as_deref()).await
        }
        Commands::Run {
            prompt,
            session,
            model,
        } => commands::run::run(&client, &prompt, session.as_deref(), model.as_deref()).await,
        Commands::Memory { action } => match action {
            MemoryAction::Search {
                query,
                limit,
                offset,
            } => commands::memory::search(&client, &query, limit, offset).await,
            MemoryAction::Add { key, content } => {
                commands::memory::add(&client, &key, &content).await
            }
            MemoryAction::Remove { key } => commands::memory::remove(&client, &key).await,
        },
        Commands::Config { action } => match action {
            ConfigAction::Show => commands::config::show(&client).await,
            ConfigAction::Set { key, value } => commands::config::set(&client, &key, &value).await,
        },
        Commands::Key { action } => match action {
            KeyAction::Set { provider, key } => commands::key::set(&client, &provider, &key).await,
            KeyAction::Remove { provider } => commands::key::remove(&client, &provider).await,
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    #[test]
    fn parse_daemon_status() {
        let cli = parse(&["mesoclaw", "daemon", "status"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon {
                action: DaemonAction::Status
            }
        ));
    }

    #[test]
    fn parse_daemon_start() {
        let cli = parse(&["mesoclaw", "daemon", "start"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon {
                action: DaemonAction::Start
            }
        ));
    }

    #[test]
    fn parse_chat_default() {
        let cli = parse(&["mesoclaw", "chat"]);
        match cli.command {
            Commands::Chat { session, model } => {
                assert!(session.is_none());
                assert!(model.is_none());
            }
            _ => panic!("expected Chat"),
        }
    }

    #[test]
    fn parse_chat_with_options() {
        let cli = parse(&["mesoclaw", "chat", "--session", "abc", "--model", "gpt-4o"]);
        match cli.command {
            Commands::Chat { session, model } => {
                assert_eq!(session.as_deref(), Some("abc"));
                assert_eq!(model.as_deref(), Some("gpt-4o"));
            }
            _ => panic!("expected Chat"),
        }
    }

    #[test]
    fn parse_run_prompt() {
        let cli = parse(&["mesoclaw", "run", "hello world"]);
        match cli.command {
            Commands::Run {
                prompt,
                session,
                model,
            } => {
                assert_eq!(prompt, "hello world");
                assert!(session.is_none());
                assert!(model.is_none());
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn parse_memory_search() {
        let cli = parse(&["mesoclaw", "memory", "search", "rust", "--limit", "5"]);
        match cli.command {
            Commands::Memory {
                action:
                    MemoryAction::Search {
                        query,
                        limit,
                        offset,
                    },
            } => {
                assert_eq!(query, "rust");
                assert_eq!(limit, Some(5));
                assert!(offset.is_none());
            }
            _ => panic!("expected Memory Search"),
        }
    }

    #[test]
    fn parse_config_set() {
        let cli = parse(&["mesoclaw", "config", "set", "log_level", "debug"]);
        match cli.command {
            Commands::Config {
                action: ConfigAction::Set { key, value },
            } => {
                assert_eq!(key, "log_level");
                assert_eq!(value, "debug");
            }
            _ => panic!("expected Config Set"),
        }
    }

    #[test]
    fn parse_global_options() {
        let cli = parse(&[
            "mesoclaw", "--host", "10.0.0.1", "--port", "9999", "--token", "secret", "daemon",
            "status",
        ]);
        assert_eq!(cli.host, "10.0.0.1");
        assert_eq!(cli.port, 9999);
        assert_eq!(cli.token, Some("secret".to_string()));
    }
}
