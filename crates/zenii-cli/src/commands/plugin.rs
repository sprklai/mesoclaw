use serde::Deserialize;

use crate::client::ZeniiClient;
use crate::commands::{encode_path_segment, truncate};

#[derive(Deserialize)]
struct PluginListItem {
    name: String,
    version: String,
    description: String,
    enabled: bool,
    tools_count: usize,
    skills_count: usize,
}

#[derive(Deserialize)]
struct PluginDetail {
    manifest: PluginManifestInfo,
    enabled: bool,
    installed_at: String,
    source: serde_json::Value,
}

#[derive(Deserialize)]
struct PluginManifestInfo {
    plugin: PluginMetaInfo,
    tools: Vec<PluginToolInfo>,
    skills: Vec<PluginSkillInfo>,
}

#[derive(Deserialize)]
struct PluginMetaInfo {
    name: String,
    version: String,
    description: String,
    author: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
}

#[derive(Deserialize)]
struct PluginToolInfo {
    name: String,
    description: String,
}

#[derive(Deserialize)]
struct PluginSkillInfo {
    name: String,
}

pub async fn list(client: &ZeniiClient) -> Result<(), String> {
    let plugins: Vec<PluginListItem> = client.get("/plugins").await?;

    if plugins.is_empty() {
        println!("No plugins installed.");
        return Ok(());
    }

    println!(
        "{:<24} {:<10} {:<5} {:<40} {:>5} {:>6}",
        "Name", "Version", "State", "Description", "Tools", "Skills"
    );
    println!("{}", "-".repeat(95));

    for p in &plugins {
        let state = if p.enabled { "on" } else { "off" };
        println!(
            "{:<24} {:<10} {:<5} {:<40} {:>5} {:>6}",
            p.name,
            p.version,
            state,
            truncate(&p.description, 38),
            p.tools_count,
            p.skills_count,
        );
    }

    println!("\n{} plugin(s)", plugins.len());
    Ok(())
}

pub async fn install(
    client: &ZeniiClient,
    source: &str,
    local: bool,
    all: bool,
) -> Result<(), String> {
    #[derive(serde::Serialize)]
    struct InstallReq<'a> {
        source: &'a str,
        local: bool,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        all: bool,
    }

    if all {
        let plugins: Vec<PluginDetail> = client
            .post("/plugins/install", &InstallReq { source, local, all })
            .await?;

        if plugins.is_empty() {
            println!("No plugins found in '{source}'");
        } else {
            for plugin in &plugins {
                println!(
                    "Installed plugin '{}' v{} ({} tool(s), {} skill(s))",
                    plugin.manifest.plugin.name,
                    plugin.manifest.plugin.version,
                    plugin.manifest.tools.len(),
                    plugin.manifest.skills.len(),
                );
            }
            println!("\n{} plugin(s) installed", plugins.len());
        }
    } else {
        let plugin: PluginDetail = client
            .post("/plugins/install", &InstallReq { source, local, all })
            .await?;

        println!(
            "Installed plugin '{}' v{} ({} tool(s), {} skill(s))",
            plugin.manifest.plugin.name,
            plugin.manifest.plugin.version,
            plugin.manifest.tools.len(),
            plugin.manifest.skills.len(),
        );
    }
    Ok(())
}

pub async fn remove(client: &ZeniiClient, name: &str) -> Result<(), String> {
    client
        .delete(&format!("/plugins/{}", encode_path_segment(name)))
        .await?;
    println!("Removed plugin '{name}'");
    Ok(())
}

pub async fn update(client: &ZeniiClient, name: &str) -> Result<(), String> {
    let plugin: PluginDetail = client
        .post(
            &format!("/plugins/{}/update", encode_path_segment(name)),
            &serde_json::json!({}),
        )
        .await?;

    println!(
        "Updated plugin '{}' to v{}",
        plugin.manifest.plugin.name, plugin.manifest.plugin.version,
    );
    Ok(())
}

pub async fn enable(client: &ZeniiClient, name: &str) -> Result<(), String> {
    let current: PluginDetail = client
        .get(&format!("/plugins/{}", encode_path_segment(name)))
        .await?;
    if current.enabled {
        println!("Plugin '{name}' is already enabled");
        return Ok(());
    }
    let plugin: PluginDetail = client
        .put(
            &format!("/plugins/{}/toggle", encode_path_segment(name)),
            &serde_json::json!({}),
        )
        .await?;
    let state = if plugin.enabled {
        "enabled"
    } else {
        "disabled"
    };
    println!("Plugin '{name}' is now {state}");
    Ok(())
}

pub async fn disable(client: &ZeniiClient, name: &str) -> Result<(), String> {
    let current: PluginDetail = client
        .get(&format!("/plugins/{}", encode_path_segment(name)))
        .await?;
    if !current.enabled {
        println!("Plugin '{name}' is already disabled");
        return Ok(());
    }
    let plugin: PluginDetail = client
        .put(
            &format!("/plugins/{}/toggle", encode_path_segment(name)),
            &serde_json::json!({}),
        )
        .await?;
    let state = if plugin.enabled {
        "enabled"
    } else {
        "disabled"
    };
    println!("Plugin '{name}' is now {state}");
    Ok(())
}

pub async fn info(client: &ZeniiClient, name: &str) -> Result<(), String> {
    let plugin: PluginDetail = client
        .get(&format!("/plugins/{}", encode_path_segment(name)))
        .await?;

    let meta = &plugin.manifest.plugin;
    println!("Name:        {}", meta.name);
    println!("Version:     {}", meta.version);
    println!("Description: {}", meta.description);
    if let Some(ref author) = meta.author {
        println!("Author:      {author}");
    }
    if let Some(ref license) = meta.license {
        println!("License:     {license}");
    }
    if let Some(ref homepage) = meta.homepage {
        println!("Homepage:    {homepage}");
    }
    println!(
        "Status:      {}",
        if plugin.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("Installed:   {}", plugin.installed_at);

    let source_str = match plugin.source.get("Git") {
        Some(git) => format!(
            "git: {}",
            git.get("url").and_then(|u| u.as_str()).unwrap_or("unknown")
        ),
        None => match plugin.source.get("Local") {
            Some(local) => format!(
                "local: {}",
                local
                    .get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown")
            ),
            None => "bundled".to_string(),
        },
    };
    println!("Source:      {source_str}");

    if !plugin.manifest.tools.is_empty() {
        println!("\nTools:");
        for tool in &plugin.manifest.tools {
            println!("  - {} — {}", tool.name, tool.description);
        }
    }

    if !plugin.manifest.skills.is_empty() {
        println!("\nSkills:");
        for skill in &plugin.manifest.skills {
            println!("  - {}", skill.name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use serde_json::json;

    use super::*;

    fn mock_plugin_json(enabled: bool) -> serde_json::Value {
        json!({
            "manifest": {
                "plugin": {
                    "name": "test-plugin",
                    "version": "1.0.0",
                    "description": "A test plugin"
                },
                "tools": [],
                "skills": []
            },
            "enabled": enabled,
            "installed_at": "2026-01-01T00:00:00Z",
            "source": "bundled"
        })
    }

    fn test_client(port: u16) -> ZeniiClient {
        ZeniiClient::new("127.0.0.1", port, None)
    }

    // enable on already-enabled plugin must NOT call toggle
    #[tokio::test]
    async fn enable_already_enabled_skips_toggle() {
        let server = MockServer::start();
        let get_mock = server.mock(|when, then| {
            when.method(GET).path("/plugins/my-plugin");
            then.status(200).json_body(mock_plugin_json(true));
        });
        let toggle_mock = server.mock(|when, then| {
            when.method(PUT).path("/plugins/my-plugin/toggle");
            then.status(200).json_body(mock_plugin_json(false));
        });

        let result = enable(&test_client(server.port()), "my-plugin").await;
        assert!(result.is_ok());
        assert_eq!(get_mock.hits(), 1);
        assert_eq!(
            toggle_mock.hits(),
            0,
            "toggle must not be called when already enabled"
        );
    }

    // disable on already-disabled plugin must NOT call toggle
    #[tokio::test]
    async fn disable_already_disabled_skips_toggle() {
        let server = MockServer::start();
        let get_mock = server.mock(|when, then| {
            when.method(GET).path("/plugins/my-plugin");
            then.status(200).json_body(mock_plugin_json(false));
        });
        let toggle_mock = server.mock(|when, then| {
            when.method(PUT).path("/plugins/my-plugin/toggle");
            then.status(200).json_body(mock_plugin_json(true));
        });

        let result = disable(&test_client(server.port()), "my-plugin").await;
        assert!(result.is_ok());
        assert_eq!(get_mock.hits(), 1);
        assert_eq!(
            toggle_mock.hits(),
            0,
            "toggle must not be called when already disabled"
        );
    }

    // enable on a disabled plugin must call GET then PUT toggle
    #[tokio::test]
    async fn enable_disabled_plugin_calls_toggle() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/plugins/my-plugin");
            then.status(200).json_body(mock_plugin_json(false));
        });
        let toggle_mock = server.mock(|when, then| {
            when.method(PUT).path("/plugins/my-plugin/toggle");
            then.status(200).json_body(mock_plugin_json(true));
        });

        let result = enable(&test_client(server.port()), "my-plugin").await;
        assert!(result.is_ok());
        assert_eq!(toggle_mock.hits(), 1);
    }

    // disable on an enabled plugin must call GET then PUT toggle
    #[tokio::test]
    async fn disable_enabled_plugin_calls_toggle() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/plugins/my-plugin");
            then.status(200).json_body(mock_plugin_json(true));
        });
        let toggle_mock = server.mock(|when, then| {
            when.method(PUT).path("/plugins/my-plugin/toggle");
            then.status(200).json_body(mock_plugin_json(false));
        });

        let result = disable(&test_client(server.port()), "my-plugin").await;
        assert!(result.is_ok());
        assert_eq!(toggle_mock.hits(), 1);
    }

    // plugin name with special chars is percent-encoded in path
    #[tokio::test]
    async fn enable_encodes_plugin_name_in_path() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/plugins/my%2Fplugin");
            then.status(200).json_body(mock_plugin_json(false));
        });
        server.mock(|when, then| {
            when.method(PUT).path("/plugins/my%2Fplugin/toggle");
            then.status(200).json_body(mock_plugin_json(true));
        });

        let result = enable(&test_client(server.port()), "my/plugin").await;
        assert!(result.is_ok());
    }
}
