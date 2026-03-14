mod client;
mod commands;

use std::process;

use clap::{CommandFactory, Parser, Subcommand};

use client::ZeniiClient;

#[derive(Parser)]
#[command(name = "zenii", about = "Zenii CLI — talk to your local AI agent")]
struct Cli {
    /// Daemon host address
    #[arg(long, default_value = "127.0.0.1", global = true)]
    host: String,

    /// Daemon port
    #[arg(long, default_value_t = 18981, global = true)]
    port: u16,

    /// Auth token (or set ZENII_TOKEN env var)
    #[arg(long, global = true, env = "ZENII_TOKEN")]
    token: Option<String>,

    /// Skip automatic setup check before chat/run
    #[arg(long, global = true)]
    no_setup: bool,

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
    /// Manage AI providers
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    /// Manage scheduled jobs
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },
    /// Manage embedding provider for semantic memory
    Embedding {
        #[command(subcommand)]
        action: EmbeddingAction,
    },
    /// Manage plugins (install, remove, update, enable, disable)
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },
    /// View channel conversations and messages
    Channel {
        #[command(subcommand)]
        action: ChannelAction,
    },
    /// Interactive onboarding wizard
    Onboard,
    /// Generate shell completions (hidden from --help)
    #[command(hide = true)]
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
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
    /// Set an API key for a provider or service (e.g. openai, tavily, brave)
    Set {
        /// Provider/service name (e.g. openai, anthropic, tavily, brave)
        provider: String,
        /// API key value
        key: String,
    },
    /// Remove an API key for a provider or service
    Remove {
        /// Provider/service name
        provider: String,
    },
    /// Set a channel credential field (e.g. telegram token, slack bot_token)
    SetChannel {
        /// Channel name (e.g. telegram, slack, discord, matrix)
        channel: String,
        /// Credential field (e.g. token, bot_token, access_token)
        field: String,
        /// Credential value
        value: String,
    },
    /// Remove a channel credential field
    RemoveChannel {
        /// Channel name
        channel: String,
        /// Credential field
        field: String,
    },
    /// Set a raw credential key (advanced: full key like channel:telegram:token)
    SetRaw {
        /// Full credential key
        key: String,
        /// Credential value
        value: String,
    },
    /// Remove a raw credential key
    RemoveRaw {
        /// Full credential key
        key: String,
    },
    /// List all stored credential keys (grouped by type)
    List,
}

#[derive(Subcommand)]
enum ScheduleAction {
    /// List all scheduled jobs
    List,
    /// Create a new scheduled job
    Create {
        /// Job name
        name: String,
        /// Schedule type: interval or cron
        #[arg(long, default_value = "interval")]
        schedule_type: String,
        /// Interval in seconds (for interval schedule)
        #[arg(long)]
        interval_secs: Option<u64>,
        /// Cron expression (for cron schedule)
        #[arg(long)]
        cron_expr: Option<String>,
        /// Payload type: heartbeat, notify, or agent_turn
        #[arg(long, default_value = "heartbeat")]
        payload: String,
        /// Message for notify payload
        #[arg(long)]
        message: Option<String>,
        /// Prompt for agent_turn payload
        #[arg(long)]
        prompt: Option<String>,
        /// Delete after first run (one-shot)
        #[arg(long)]
        one_shot: bool,
    },
    /// Toggle a job enabled/disabled
    Toggle {
        /// Job ID
        id: String,
    },
    /// Delete a scheduled job
    Delete {
        /// Job ID
        id: String,
    },
    /// Show execution history for a job
    History {
        /// Job ID
        id: String,
    },
    /// Show scheduler status
    Status,
}

#[derive(Subcommand)]
enum PluginAction {
    /// List installed plugins
    List,
    /// Install a plugin from a git URL or local path
    Install {
        /// Git URL or local path to the plugin
        source: String,
        /// Install from a local directory instead of git
        #[arg(long)]
        local: bool,
    },
    /// Remove an installed plugin
    Remove {
        /// Plugin name
        name: String,
    },
    /// Update a git-installed plugin to the latest version
    Update {
        /// Plugin name
        name: String,
    },
    /// Enable a disabled plugin
    Enable {
        /// Plugin name
        name: String,
    },
    /// Disable a plugin without removing it
    Disable {
        /// Plugin name
        name: String,
    },
    /// Show detailed plugin information
    Info {
        /// Plugin name
        name: String,
    },
}

#[derive(Subcommand)]
enum ChannelAction {
    /// List channel conversations
    List {
        /// Filter by channel source (telegram, slack, discord)
        #[arg(long)]
        source: Option<String>,
    },
    /// View messages in a channel conversation
    Messages {
        /// Session ID
        session_id: String,
        /// Maximum messages to show
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Cursor: show messages before this message ID
        #[arg(long)]
        before: Option<String>,
    },
}

#[derive(Subcommand)]
enum EmbeddingAction {
    /// Show current embedding provider status
    Status,
    /// Activate an embedding provider (local or openai)
    Activate {
        /// Provider type: "local" or "openai"
        provider: String,
    },
    /// Deactivate embedding provider (FTS5 only)
    Deactivate,
    /// Download local embedding model
    Download,
    /// Test embedding provider with sample text
    Test,
    /// Re-embed all memories with current provider
    Reindex,
}

#[derive(Subcommand)]
enum ProviderAction {
    /// List all providers with key status
    List,
    /// Test connection to a provider
    Test {
        /// Provider ID (e.g. openai, anthropic)
        provider_id: String,
    },
    /// Add a custom provider
    Add {
        /// Provider ID (alphanumeric + hyphens)
        id: String,
        /// Display name
        #[arg(long)]
        name: Option<String>,
        /// Base URL for the API
        #[arg(long)]
        base_url: String,
    },
    /// Remove a user-defined provider
    Remove {
        /// Provider ID
        provider_id: String,
    },
    /// Set the default model
    Default {
        /// Provider ID
        provider_id: String,
        /// Model ID
        model_id: String,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let client = ZeniiClient::new(&cli.host, cli.port, cli.token);

    // Auto-trigger onboarding for interactive commands if setup is incomplete
    if !cli.no_setup
        && std::io::IsTerminal::is_terminal(&std::io::stdin())
        && matches!(cli.command, Commands::Chat { .. } | Commands::Run { .. })
        && let Ok(status) = client.get::<serde_json::Value>("/setup/status").await
        && status.get("needs_setup").and_then(|v| v.as_bool()) == Some(true)
        && let Err(e) = commands::onboard::run(&client).await
    {
        eprintln!("Onboarding failed: {e}");
    }

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
            KeyAction::SetChannel {
                channel,
                field,
                value,
            } => commands::key::set_channel(&client, &channel, &field, &value).await,
            KeyAction::RemoveChannel { channel, field } => {
                commands::key::remove_channel(&client, &channel, &field).await
            }
            KeyAction::SetRaw { key, value } => commands::key::set_raw(&client, &key, &value).await,
            KeyAction::RemoveRaw { key } => commands::key::remove_raw(&client, &key).await,
            KeyAction::List => commands::key::list(&client).await,
        },
        Commands::Schedule { action } => match action {
            ScheduleAction::List => commands::schedule::list(&client).await,
            ScheduleAction::Create {
                name,
                schedule_type,
                interval_secs,
                cron_expr,
                payload,
                message,
                prompt,
                one_shot,
            } => {
                commands::schedule::create(
                    &client,
                    commands::schedule::CreateJobArgs {
                        name: &name,
                        schedule_type: &schedule_type,
                        interval_secs,
                        cron_expr: cron_expr.as_deref(),
                        payload_type: &payload,
                        message: message.as_deref(),
                        prompt: prompt.as_deref(),
                        one_shot,
                    },
                )
                .await
            }
            ScheduleAction::Toggle { id } => commands::schedule::toggle(&client, &id).await,
            ScheduleAction::Delete { id } => commands::schedule::delete(&client, &id).await,
            ScheduleAction::History { id } => commands::schedule::history(&client, &id).await,
            ScheduleAction::Status => commands::schedule::status(&client).await,
        },
        Commands::Embedding { action } => match action {
            EmbeddingAction::Status => commands::embedding::status(&client).await,
            EmbeddingAction::Activate { provider } => {
                commands::embedding::activate(&client, &provider).await
            }
            EmbeddingAction::Deactivate => commands::embedding::deactivate(&client).await,
            EmbeddingAction::Download => commands::embedding::download(&client).await,
            EmbeddingAction::Test => commands::embedding::test(&client).await,
            EmbeddingAction::Reindex => commands::embedding::reindex(&client).await,
        },
        Commands::Plugin { action } => match action {
            PluginAction::List => commands::plugin::list(&client).await,
            PluginAction::Install { source, local } => {
                commands::plugin::install(&client, &source, local).await
            }
            PluginAction::Remove { name } => commands::plugin::remove(&client, &name).await,
            PluginAction::Update { name } => commands::plugin::update(&client, &name).await,
            PluginAction::Enable { name } => commands::plugin::enable(&client, &name).await,
            PluginAction::Disable { name } => commands::plugin::disable(&client, &name).await,
            PluginAction::Info { name } => commands::plugin::info(&client, &name).await,
        },
        Commands::Channel { action } => match action {
            ChannelAction::List { source } => {
                commands::channel::list(&client, source.as_deref()).await
            }
            ChannelAction::Messages {
                session_id,
                limit,
                before,
            } => commands::channel::messages(&client, &session_id, limit, before.as_deref()).await,
        },
        Commands::Onboard => commands::onboard::run(&client).await,
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "zenii", &mut std::io::stdout());
            Ok(())
        }
        Commands::Provider { action } => match action {
            ProviderAction::List => commands::provider::list(&client).await,
            ProviderAction::Test { provider_id } => {
                commands::provider::test_connection(&client, &provider_id).await
            }
            ProviderAction::Add { id, name, base_url } => {
                let display_name = name.as_deref().unwrap_or(&id);
                commands::provider::add(&client, &id, display_name, &base_url).await
            }
            ProviderAction::Remove { provider_id } => {
                commands::provider::remove(&client, &provider_id).await
            }
            ProviderAction::Default {
                provider_id,
                model_id,
            } => commands::provider::set_default(&client, &provider_id, &model_id).await,
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
        let cli = parse(&["zenii", "daemon", "status"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon {
                action: DaemonAction::Status
            }
        ));
    }

    #[test]
    fn parse_daemon_start() {
        let cli = parse(&["zenii", "daemon", "start"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon {
                action: DaemonAction::Start
            }
        ));
    }

    #[test]
    fn parse_chat_default() {
        let cli = parse(&["zenii", "chat"]);
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
        let cli = parse(&["zenii", "chat", "--session", "abc", "--model", "gpt-4o"]);
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
        let cli = parse(&["zenii", "run", "hello world"]);
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
        let cli = parse(&["zenii", "memory", "search", "rust", "--limit", "5"]);
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
        let cli = parse(&["zenii", "config", "set", "log_level", "debug"]);
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
            "zenii", "--host", "10.0.0.1", "--port", "9999", "--token", "secret", "daemon",
            "status",
        ]);
        assert_eq!(cli.host, "10.0.0.1");
        assert_eq!(cli.port, 9999);
        assert_eq!(cli.token, Some("secret".to_string()));
    }

    #[test]
    fn parse_key_list() {
        let cli = parse(&["zenii", "key", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Key {
                action: KeyAction::List
            }
        ));
    }

    #[test]
    fn parse_key_set() {
        let cli = parse(&["zenii", "key", "set", "tavily", "tvly-123"]);
        match cli.command {
            Commands::Key {
                action: KeyAction::Set { provider, key },
            } => {
                assert_eq!(provider, "tavily");
                assert_eq!(key, "tvly-123");
            }
            _ => panic!("expected Key Set"),
        }
    }

    #[test]
    fn parse_key_set_channel() {
        let cli = parse(&[
            "zenii",
            "key",
            "set-channel",
            "telegram",
            "token",
            "bot123:abc",
        ]);
        match cli.command {
            Commands::Key {
                action:
                    KeyAction::SetChannel {
                        channel,
                        field,
                        value,
                    },
            } => {
                assert_eq!(channel, "telegram");
                assert_eq!(field, "token");
                assert_eq!(value, "bot123:abc");
            }
            _ => panic!("expected Key SetChannel"),
        }
    }

    #[test]
    fn parse_key_remove_channel() {
        let cli = parse(&["zenii", "key", "remove-channel", "slack", "bot_token"]);
        match cli.command {
            Commands::Key {
                action: KeyAction::RemoveChannel { channel, field },
            } => {
                assert_eq!(channel, "slack");
                assert_eq!(field, "bot_token");
            }
            _ => panic!("expected Key RemoveChannel"),
        }
    }

    #[test]
    fn parse_key_set_raw() {
        let cli = parse(&["zenii", "key", "set-raw", "custom:key", "val"]);
        match cli.command {
            Commands::Key {
                action: KeyAction::SetRaw { key, value },
            } => {
                assert_eq!(key, "custom:key");
                assert_eq!(value, "val");
            }
            _ => panic!("expected Key SetRaw"),
        }
    }

    #[test]
    fn parse_provider_list() {
        let cli = parse(&["zenii", "provider", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Provider {
                action: ProviderAction::List
            }
        ));
    }

    #[test]
    fn parse_provider_test() {
        let cli = parse(&["zenii", "provider", "test", "openai"]);
        match cli.command {
            Commands::Provider {
                action: ProviderAction::Test { provider_id },
            } => {
                assert_eq!(provider_id, "openai");
            }
            _ => panic!("expected Provider Test"),
        }
    }

    #[test]
    fn parse_provider_default() {
        let cli = parse(&["zenii", "provider", "default", "openai", "gpt-4o"]);
        match cli.command {
            Commands::Provider {
                action:
                    ProviderAction::Default {
                        provider_id,
                        model_id,
                    },
            } => {
                assert_eq!(provider_id, "openai");
                assert_eq!(model_id, "gpt-4o");
            }
            _ => panic!("expected Provider Default"),
        }
    }

    #[test]
    fn parse_completions_bash() {
        let cli = parse(&["zenii", "completions", "bash"]);
        assert!(matches!(cli.command, Commands::Completions { .. }));
    }

    #[test]
    fn parse_schedule_list() {
        let cli = parse(&["zenii", "schedule", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Schedule {
                action: ScheduleAction::List
            }
        ));
    }

    #[test]
    fn parse_schedule_status() {
        let cli = parse(&["zenii", "schedule", "status"]);
        assert!(matches!(
            cli.command,
            Commands::Schedule {
                action: ScheduleAction::Status
            }
        ));
    }

    #[test]
    fn parse_schedule_create_interval() {
        let cli = parse(&[
            "zenii",
            "schedule",
            "create",
            "my-job",
            "--schedule-type",
            "interval",
            "--interval-secs",
            "300",
            "--payload",
            "heartbeat",
        ]);
        match cli.command {
            Commands::Schedule {
                action:
                    ScheduleAction::Create {
                        name,
                        schedule_type,
                        interval_secs,
                        payload,
                        one_shot,
                        ..
                    },
            } => {
                assert_eq!(name, "my-job");
                assert_eq!(schedule_type, "interval");
                assert_eq!(interval_secs, Some(300));
                assert_eq!(payload, "heartbeat");
                assert!(!one_shot);
            }
            _ => panic!("expected Schedule Create"),
        }
    }

    #[test]
    fn parse_schedule_toggle() {
        let cli = parse(&["zenii", "schedule", "toggle", "job-123"]);
        match cli.command {
            Commands::Schedule {
                action: ScheduleAction::Toggle { id },
            } => {
                assert_eq!(id, "job-123");
            }
            _ => panic!("expected Schedule Toggle"),
        }
    }

    #[test]
    fn parse_schedule_delete() {
        let cli = parse(&["zenii", "schedule", "delete", "job-456"]);
        match cli.command {
            Commands::Schedule {
                action: ScheduleAction::Delete { id },
            } => {
                assert_eq!(id, "job-456");
            }
            _ => panic!("expected Schedule Delete"),
        }
    }

    #[test]
    fn parse_schedule_history() {
        let cli = parse(&["zenii", "schedule", "history", "job-789"]);
        match cli.command {
            Commands::Schedule {
                action: ScheduleAction::History { id },
            } => {
                assert_eq!(id, "job-789");
            }
            _ => panic!("expected Schedule History"),
        }
    }

    // IN.18 — parse channel list
    #[test]
    fn parse_channel_list() {
        let cli = parse(&["zenii", "channel", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Channel {
                action: ChannelAction::List { source: None }
            }
        ));
    }

    // IN.19 — parse channel list with source filter
    #[test]
    fn parse_channel_list_with_source() {
        let cli = parse(&["zenii", "channel", "list", "--source", "telegram"]);
        match cli.command {
            Commands::Channel {
                action: ChannelAction::List { source },
            } => {
                assert_eq!(source.as_deref(), Some("telegram"));
            }
            _ => panic!("expected Channel List"),
        }
    }

    // IN.20 — parse channel messages
    #[test]
    fn parse_channel_messages() {
        let cli = parse(&["zenii", "channel", "messages", "sess-123"]);
        match cli.command {
            Commands::Channel {
                action:
                    ChannelAction::Messages {
                        session_id,
                        limit,
                        before,
                    },
            } => {
                assert_eq!(session_id, "sess-123");
                assert_eq!(limit, 50);
                assert!(before.is_none());
            }
            _ => panic!("expected Channel Messages"),
        }
    }

    // IN.21 — parse channel messages with options
    #[test]
    fn parse_channel_messages_with_options() {
        let cli = parse(&[
            "zenii", "channel", "messages", "sess-abc", "--limit", "20", "--before", "msg-xyz",
        ]);
        match cli.command {
            Commands::Channel {
                action:
                    ChannelAction::Messages {
                        session_id,
                        limit,
                        before,
                    },
            } => {
                assert_eq!(session_id, "sess-abc");
                assert_eq!(limit, 20);
                assert_eq!(before.as_deref(), Some("msg-xyz"));
            }
            _ => panic!("expected Channel Messages"),
        }
    }

    // 9.0 — parse plugin list
    #[test]
    fn parse_plugin_list() {
        let cli = parse(&["zenii", "plugin", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Plugin {
                action: PluginAction::List
            }
        ));
    }

    // 9.0 — parse plugin install from git
    #[test]
    fn parse_plugin_install_git() {
        let cli = parse(&["zenii", "plugin", "install", "github.com/user/weather"]);
        match cli.command {
            Commands::Plugin {
                action: PluginAction::Install { source, local },
            } => {
                assert_eq!(source, "github.com/user/weather");
                assert!(!local);
            }
            _ => panic!("expected Plugin Install"),
        }
    }

    // 9.0 — parse plugin install from local
    #[test]
    fn parse_plugin_install_local() {
        let cli = parse(&["zenii", "plugin", "install", "./my-plugin", "--local"]);
        match cli.command {
            Commands::Plugin {
                action: PluginAction::Install { source, local },
            } => {
                assert_eq!(source, "./my-plugin");
                assert!(local);
            }
            _ => panic!("expected Plugin Install Local"),
        }
    }

    // 9.0 — parse plugin remove
    #[test]
    fn parse_plugin_remove() {
        let cli = parse(&["zenii", "plugin", "remove", "weather"]);
        match cli.command {
            Commands::Plugin {
                action: PluginAction::Remove { name },
            } => {
                assert_eq!(name, "weather");
            }
            _ => panic!("expected Plugin Remove"),
        }
    }

    // 9.0 — parse plugin info
    #[test]
    fn parse_plugin_info() {
        let cli = parse(&["zenii", "plugin", "info", "weather"]);
        match cli.command {
            Commands::Plugin {
                action: PluginAction::Info { name },
            } => {
                assert_eq!(name, "weather");
            }
            _ => panic!("expected Plugin Info"),
        }
    }

    // 9.0 — parse plugin enable/disable
    #[test]
    fn parse_plugin_enable_disable() {
        let cli = parse(&["zenii", "plugin", "enable", "weather"]);
        match cli.command {
            Commands::Plugin {
                action: PluginAction::Enable { name },
            } => {
                assert_eq!(name, "weather");
            }
            _ => panic!("expected Plugin Enable"),
        }

        let cli = parse(&["zenii", "plugin", "disable", "docker"]);
        match cli.command {
            Commands::Plugin {
                action: PluginAction::Disable { name },
            } => {
                assert_eq!(name, "docker");
            }
            _ => panic!("expected Plugin Disable"),
        }
    }

    // 9.0 — parse plugin update
    #[test]
    fn parse_plugin_update() {
        let cli = parse(&["zenii", "plugin", "update", "rss"]);
        match cli.command {
            Commands::Plugin {
                action: PluginAction::Update { name },
            } => {
                assert_eq!(name, "rss");
            }
            _ => panic!("expected Plugin Update"),
        }
    }

    #[test]
    fn parse_onboard() {
        let cli = parse(&["zenii", "onboard"]);
        assert!(matches!(cli.command, Commands::Onboard));
    }

    #[test]
    fn parse_no_setup_flag() {
        let cli = parse(&["zenii", "--no-setup", "chat"]);
        assert!(cli.no_setup);
    }
}
