use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::generated_prompts;

/// Generated prompt artifact (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = generated_prompts)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPrompt {
    pub id: String,
    pub name: String,
    pub artifact_type: String,
    pub content: String,
    pub disk_path: Option<String>,
    pub created_at: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}

/// New generated prompt for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = generated_prompts)]
pub struct NewGeneratedPrompt {
    pub id: String,
    pub name: String,
    pub artifact_type: String,
    pub content: String,
    pub disk_path: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}
