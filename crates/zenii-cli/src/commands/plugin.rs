use serde::Deserialize;

use crate::client::ZeniiClient;

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
        .delete(&format!("/plugins/{}", urlencoded(name)))
        .await?;
    println!("Removed plugin '{name}'");
    Ok(())
}

pub async fn update(client: &ZeniiClient, name: &str) -> Result<(), String> {
    let plugin: PluginDetail = client
        .post(
            &format!("/plugins/{}/update", urlencoded(name)),
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
    let plugin: PluginDetail = client
        .put(
            &format!("/plugins/{}/toggle", urlencoded(name)),
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
    // toggle endpoint flips state — call once to disable
    enable(client, name).await
}

pub async fn info(client: &ZeniiClient, name: &str) -> Result<(), String> {
    let plugin: PluginDetail = client
        .get(&format!("/plugins/{}", urlencoded(name)))
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
}
