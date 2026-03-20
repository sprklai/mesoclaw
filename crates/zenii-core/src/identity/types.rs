use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdentityMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl Default for IdentityMeta {
    fn default() -> Self {
        Self {
            name: "Zenii".into(),
            version: "0.0.33".into(),
            description: "AI-powered assistant for developers".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaFile {
    pub name: String,
    pub description: String,
    pub content: String,
    pub is_default: bool,
}

impl PersonaFile {
    pub fn new(name: impl Into<String>, content: impl Into<String>, is_default: bool) -> Self {
        let name = name.into();
        Self {
            description: format!("{name} identity file"),
            name,
            content: content.into(),
            is_default,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub meta: IdentityMeta,
    pub files: HashMap<String, PersonaFile>,
}

impl Identity {
    pub fn new(meta: IdentityMeta) -> Self {
        Self {
            meta,
            files: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_meta_default() {
        let meta = IdentityMeta::default();
        assert_eq!(meta.name, "Zenii");
        assert_eq!(meta.version, "0.0.33");
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn persona_file_new() {
        let pf = PersonaFile::new("SOUL", "content here", true);
        assert_eq!(pf.name, "SOUL");
        assert_eq!(pf.content, "content here");
        assert!(pf.is_default);
        assert!(pf.description.contains("SOUL"));
    }

    #[test]
    fn identity_new() {
        let identity = Identity::new(IdentityMeta::default());
        assert_eq!(identity.meta.name, "Zenii");
        assert!(identity.files.is_empty());
    }

    #[test]
    fn identity_meta_serialize_roundtrip() {
        let meta = IdentityMeta::default();
        let yaml = serde_yaml::to_string(&meta).unwrap();
        let parsed: IdentityMeta = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(meta, parsed);
    }
}
