use crate::config::AppConfig;

use super::types::Identity;

/// Assembles a dynamic system prompt from identity files, skills, observations, and config.
pub struct PromptComposer;

impl PromptComposer {
    /// Compose the full system prompt in the prescribed order:
    /// 1. SOUL.md content
    /// 2. IDENTITY.md metadata (name, version)
    /// 3. USER.md content
    /// 4. User observations
    /// 5. Active skills
    /// 6. agent_system_prompt config override (appended)
    pub fn compose(
        identity: &Identity,
        active_skills: &[(String, String)], // (name, content)
        observations: &str,
        config: &AppConfig,
    ) -> String {
        let mut parts = Vec::new();

        // 1. SOUL
        if let Some(soul) = identity.files.get("SOUL")
            && !soul.content.trim().is_empty()
        {
            parts.push(soul.content.clone());
        }

        // 2. IDENTITY metadata
        parts.push(format!(
            "## Agent Identity\n- Name: {}\n- Version: {}\n- Description: {}",
            identity.meta.name, identity.meta.version, identity.meta.description
        ));

        // 3. USER
        if let Some(user) = identity.files.get("USER")
            && !user.content.trim().is_empty()
        {
            parts.push(user.content.clone());
        }

        // 4. Observations
        if !observations.trim().is_empty() {
            parts.push(format!("## Known Preferences\n{observations}"));
        }

        // 5. Skills
        if !active_skills.is_empty() {
            let mut skills_section = String::from("## Active Skills\n");
            for (name, content) in active_skills {
                skills_section.push_str(&format!("\n### {name}\n{content}\n"));
            }
            parts.push(skills_section);
        }

        // 6. Config override (append, not replace)
        if let Some(ref override_prompt) = config.agent_system_prompt
            && !override_prompt.trim().is_empty()
        {
            parts.push(override_prompt.clone());
        }

        parts.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::types::{IdentityMeta, PersonaFile};

    fn test_identity() -> Identity {
        let mut identity = Identity::new(IdentityMeta {
            name: "TestBot".into(),
            version: "1.0".into(),
            description: "A test bot".into(),
        });
        identity.files.insert(
            "SOUL".into(),
            PersonaFile::new("SOUL", "You are a helpful assistant.", true),
        );
        identity.files.insert(
            "IDENTITY".into(),
            PersonaFile::new("IDENTITY", "---\nname: TestBot\n---\n# Identity", true),
        );
        identity.files.insert(
            "USER".into(),
            PersonaFile::new("USER", "User is a Rust developer.", true),
        );
        identity
    }

    #[test]
    fn compose_includes_soul() {
        let identity = test_identity();
        let config = AppConfig::default();
        let result = PromptComposer::compose(&identity, &[], "", &config);
        assert!(result.contains("helpful assistant"));
    }

    #[test]
    fn compose_includes_identity_meta() {
        let identity = test_identity();
        let config = AppConfig::default();
        let result = PromptComposer::compose(&identity, &[], "", &config);
        assert!(result.contains("TestBot"));
        assert!(result.contains("1.0"));
    }

    #[test]
    fn compose_includes_user() {
        let identity = test_identity();
        let config = AppConfig::default();
        let result = PromptComposer::compose(&identity, &[], "", &config);
        assert!(result.contains("Rust developer"));
    }

    #[test]
    fn compose_includes_observations() {
        let identity = test_identity();
        let config = AppConfig::default();
        let observations = "- Prefers concise answers\n- Uses vim";
        let result = PromptComposer::compose(&identity, &[], observations, &config);
        assert!(result.contains("Known Preferences"));
        assert!(result.contains("Prefers concise answers"));
    }

    #[test]
    fn compose_includes_skills() {
        let identity = test_identity();
        let config = AppConfig::default();
        let skills = vec![("coding".into(), "Write clean code.".into())];
        let result = PromptComposer::compose(&identity, &skills, "", &config);
        assert!(result.contains("Active Skills"));
        assert!(result.contains("coding"));
        assert!(result.contains("Write clean code."));
    }

    #[test]
    fn compose_appends_config_override() {
        let identity = test_identity();
        let config = AppConfig {
            agent_system_prompt: Some("Always respond in JSON.".into()),
            ..Default::default()
        };
        let result = PromptComposer::compose(&identity, &[], "", &config);
        assert!(result.contains("Always respond in JSON."));
        // Override should be at the end
        let json_pos = result.find("Always respond in JSON.").unwrap();
        let soul_pos = result.find("helpful assistant").unwrap();
        assert!(json_pos > soul_pos);
    }

    #[test]
    fn compose_order_is_correct() {
        let identity = test_identity();
        let config = AppConfig {
            agent_system_prompt: Some("OVERRIDE".into()),
            ..Default::default()
        };
        let skills = vec![("test-skill".into(), "Skill content".into())];
        let observations = "User likes tests";
        let result = PromptComposer::compose(&identity, &skills, observations, &config);

        let soul_pos = result.find("helpful assistant").unwrap();
        let meta_pos = result.find("Agent Identity").unwrap();
        let user_pos = result.find("Rust developer").unwrap();
        let obs_pos = result.find("Known Preferences").unwrap();
        let skill_pos = result.find("Active Skills").unwrap();
        let override_pos = result.find("OVERRIDE").unwrap();

        assert!(soul_pos < meta_pos);
        assert!(meta_pos < user_pos);
        assert!(user_pos < obs_pos);
        assert!(obs_pos < skill_pos);
        assert!(skill_pos < override_pos);
    }

    #[test]
    fn compose_empty_observations() {
        let identity = test_identity();
        let config = AppConfig::default();
        let result = PromptComposer::compose(&identity, &[], "", &config);
        assert!(!result.contains("Known Preferences"));
    }
}
