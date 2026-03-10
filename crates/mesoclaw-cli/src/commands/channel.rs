use serde::Deserialize;

use crate::client::MesoClient;

#[derive(Deserialize)]
struct ChannelSession {
    id: String,
    title: String,
    source: String,
    message_count: i64,
    updated_at: String,
}

#[derive(Deserialize)]
struct ChannelMessage {
    role: String,
    content: String,
    created_at: String,
}

pub async fn list(client: &MesoClient, source: Option<&str>) -> Result<(), String> {
    let mut path = "/channels/sessions?limit=50".to_string();
    if let Some(src) = source {
        path.push_str(&format!("&source={src}"));
    }

    let sessions: Vec<ChannelSession> = client.get(&path).await?;

    if sessions.is_empty() {
        println!("No channel conversations found.");
        return Ok(());
    }

    println!(
        "{:<38} {:<10} {:<30} {:>6}  {:<20}",
        "ID", "Source", "Title", "Msgs", "Updated"
    );
    println!("{}", "-".repeat(100));

    for s in &sessions {
        println!(
            "{:<38} {:<10} {:<30} {:>6}  {:<20}",
            s.id,
            s.source,
            truncate(&s.title, 28),
            s.message_count,
            &s.updated_at[..19.min(s.updated_at.len())]
        );
    }

    println!("\n{} conversation(s)", sessions.len());
    Ok(())
}

pub async fn messages(
    client: &MesoClient,
    session_id: &str,
    limit: usize,
    before: Option<&str>,
) -> Result<(), String> {
    let mut path = format!("/channels/sessions/{session_id}/messages?limit={limit}");
    if let Some(bid) = before {
        path.push_str(&format!("&before={bid}"));
    }

    let messages: Vec<ChannelMessage> = client.get(&path).await?;

    if messages.is_empty() {
        println!("No messages found.");
        return Ok(());
    }

    for msg in &messages {
        let time = &msg.created_at[..19.min(msg.created_at.len())];
        let role_label = match msg.role.as_str() {
            "user" => "USER",
            "assistant" => "BOT ",
            _ => &msg.role,
        };
        println!("[{time}] {role_label}: {}", msg.content);
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
