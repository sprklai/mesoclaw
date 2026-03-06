use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub gateway_host: String,
    pub gateway_port: u16,
    pub log_level: String,
    pub data_dir: Option<String>,
    pub db_path: Option<String>,
    pub memory_db_path: Option<String>,
    pub identity_name: String,
    pub identity_description: String,
    pub default_provider: String,
    pub default_model: String,
    pub security_autonomy_level: String,
    pub max_tool_retries: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway_host: "127.0.0.1".into(),
            gateway_port: 18981,
            log_level: "info".into(),
            data_dir: None,
            db_path: None,
            memory_db_path: None,
            identity_name: "MesoClaw".into(),
            identity_description: "AI-powered assistant".into(),
            default_provider: "openai".into(),
            default_model: "gpt-4o".into(),
            security_autonomy_level: "supervised".into(),
            max_tool_retries: 3,
        }
    }
}
