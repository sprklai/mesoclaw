use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum ObservationCategory {
    Preference,
    Workflow,
    Communication,
    Technical,
    Custom(String),
}

impl std::fmt::Display for ObservationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preference => write!(f, "preference"),
            Self::Workflow => write!(f, "workflow"),
            Self::Communication => write!(f, "communication"),
            Self::Technical => write!(f, "technical"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl From<&str> for ObservationCategory {
    fn from(s: &str) -> Self {
        match s {
            "preference" => Self::Preference,
            "workflow" => Self::Workflow,
            "communication" => Self::Communication,
            "technical" => Self::Technical,
            other => Self::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct UserObservation {
    pub id: String,
    pub category: String,
    pub key: String,
    pub value: String,
    pub confidence: f32,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_category_display() {
        assert_eq!(ObservationCategory::Preference.to_string(), "preference");
        assert_eq!(ObservationCategory::Workflow.to_string(), "workflow");
        assert_eq!(
            ObservationCategory::Custom("custom-cat".into()).to_string(),
            "custom-cat"
        );
    }

    #[test]
    fn user_observation_serialize_roundtrip() {
        let obs = UserObservation {
            id: "obs-1".into(),
            category: "preference".into(),
            key: "editor".into(),
            value: "vim".into(),
            confidence: 0.9,
            created_at: "2026-01-01T00:00:00".into(),
            updated_at: "2026-01-01T00:00:00".into(),
        };
        let json = serde_json::to_string(&obs).unwrap();
        let parsed: UserObservation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key, "editor");
        assert_eq!(parsed.confidence, 0.9);
    }
}
