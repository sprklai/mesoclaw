use dialoguer::{Confirm, Input, Password, Select};
use serde::Deserialize;
use serde_json::json;

use crate::client::ZeniiClient;

#[derive(Deserialize)]
struct ProviderInfo {
    id: String,
    name: String,
    requires_api_key: bool,
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    model_id: String,
    display_name: String,
}

struct ChannelDef {
    id: &'static str,
    name: &'static str,
    credentials: &'static [(&'static str, &'static str, bool)], // (key, label, is_secret)
}

const CHANNELS: &[ChannelDef] = &[
    ChannelDef {
        id: "telegram",
        name: "Telegram",
        credentials: &[
            ("token", "Bot Token (from @BotFather)", true),
            (
                "allowed_chat_ids",
                "Allowed Chat IDs (comma-separated, empty = all)",
                false,
            ),
        ],
    },
    ChannelDef {
        id: "slack",
        name: "Slack",
        credentials: &[
            ("bot_token", "Bot Token (xoxb-...)", true),
            ("app_token", "App Token (xapp-...)", true),
            (
                "allowed_channel_ids",
                "Allowed Channel IDs (comma-separated, empty = all)",
                false,
            ),
        ],
    },
    ChannelDef {
        id: "discord",
        name: "Discord",
        credentials: &[
            ("token", "Bot Token", true),
            (
                "allowed_guild_ids",
                "Allowed Server IDs (comma-separated, empty = all)",
                false,
            ),
            (
                "allowed_channel_ids",
                "Allowed Channel IDs (comma-separated, empty = all)",
                false,
            ),
        ],
    },
];

pub async fn run(client: &ZeniiClient) -> Result<(), String> {
    println!("\nWelcome to Zenii! Let's get you set up.\n");

    // Step 1: Fetch providers
    let providers: Vec<ProviderInfo> = client
        .get("/providers/with-key-status")
        .await
        .map_err(|e| format!("Failed to fetch providers: {e}"))?;

    if providers.is_empty() {
        return Err("No AI providers available. Check your daemon configuration.".into());
    }

    // Step 1: Select provider
    let provider_names: Vec<String> = providers.iter().map(|p| p.name.clone()).collect();
    let selection = Select::new()
        .with_prompt("Choose your AI provider")
        .items(&provider_names)
        .default(0)
        .interact()
        .map_err(|e| e.to_string())?;

    let selected_id = providers[selection].id.clone();
    let selected_name = providers[selection].name.clone();
    let selected_requires_key = providers[selection].requires_api_key;
    println!();

    // Step 1: API key (skip for providers that don't need it, like Ollama)
    if selected_requires_key {
        let api_key: String = Password::new()
            .with_prompt(format!("Enter your {} API key", selected_name))
            .interact()
            .map_err(|e| e.to_string())?;

        let body = json!({
            "key": format!("api_key:{}", selected_id),
            "value": api_key,
        });
        let _resp: serde_json::Value = client.post("/credentials", &body).await?;
        println!("  API key saved for {}\n", selected_name);
    } else {
        println!("  {} does not require an API key\n", selected_name);
    }

    // Step 1: Refresh providers to get updated models list
    let providers: Vec<ProviderInfo> = client
        .get("/providers/with-key-status")
        .await
        .unwrap_or(providers);

    let provider = providers
        .iter()
        .find(|p| p.id == selected_id)
        .unwrap_or(&providers[selection]);

    // Step 1: Select model (if models available)
    if !provider.models.is_empty() {
        let model_names: Vec<String> = provider
            .models
            .iter()
            .map(|m| m.display_name.clone())
            .collect();
        let model_selection = Select::new()
            .with_prompt("Select a model")
            .items(&model_names)
            .default(0)
            .interact()
            .map_err(|e| e.to_string())?;

        let model = &provider.models[model_selection];
        let body = json!({
            "provider_id": provider.id,
            "model_id": model.model_id,
        });
        let _resp: serde_json::Value = client.put("/providers/default", &body).await?;

        // Sync default model to config.toml
        let config_body = json!({
            "provider_name": provider.id,
            "provider_type": provider.id,
            "provider_model_id": model.model_id,
        });
        let _: serde_json::Value = client.put("/config", &config_body).await?;

        println!(
            "  Default model set to {}:{}\n",
            provider.id, model.model_id
        );
    }

    // Step 2: Channels (optional)
    let setup_channels = Confirm::new()
        .with_prompt("Set up messaging channels (Telegram, Slack, Discord)? (optional)")
        .default(false)
        .interact()
        .map_err(|e| e.to_string())?;

    if setup_channels {
        println!();
        let channel_names: Vec<&str> = CHANNELS.iter().map(|c| c.name).collect();
        let channel_selection = Select::new()
            .with_prompt("Choose a channel to configure")
            .items(&channel_names)
            .default(0)
            .interact()
            .map_err(|e| e.to_string())?;

        let channel = &CHANNELS[channel_selection];
        println!("\n  Configuring {}...\n", channel.name);

        for &(key, label, is_secret) in channel.credentials {
            let value = if is_secret {
                Password::new()
                    .with_prompt(label)
                    .allow_empty_password(true)
                    .interact()
                    .map_err(|e| e.to_string())?
            } else {
                Input::<String>::new()
                    .with_prompt(label)
                    .allow_empty(true)
                    .interact_text()
                    .map_err(|e| e.to_string())?
            };

            if !value.trim().is_empty() {
                let body = json!({
                    "key": format!("channel:{}:{}", channel.id, key),
                    "value": value.trim(),
                });
                let _resp: serde_json::Value = client.post("/credentials", &body).await?;
                println!("  Saved {}\n", label);
            }
        }
    }

    // Step 3: User profile
    println!();
    let name: String = Input::new()
        .with_prompt("Your name")
        .interact_text()
        .map_err(|e| e.to_string())?;

    let location: String = Input::new()
        .with_prompt("Your location (e.g., Toronto, Canada)")
        .interact_text()
        .map_err(|e| e.to_string())?;

    let detected_tz = iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".into());
    let timezone: String = Input::new()
        .with_prompt("Timezone")
        .default(detected_tz)
        .interact_text()
        .map_err(|e| e.to_string())?;

    // Step 3: Save profile
    let body = json!({
        "user_name": name,
        "user_location": location,
        "user_timezone": timezone,
    });
    let _resp: serde_json::Value = client.put("/config", &body).await?;

    println!("\n  Setup complete! Run `zenii chat` to start chatting.\n");
    Ok(())
}
