use std::path::{Path, PathBuf};

use tokio::sync::RwLock;
use tracing::info;

use crate::{MesoError, Result};

use super::defaults;
use super::types::{Identity, IdentityMeta, PersonaFile};

const IDENTITY_FILES: &[(&str, &str)] = &[
    ("SOUL", defaults::DEFAULT_SOUL),
    ("IDENTITY", defaults::DEFAULT_IDENTITY),
    ("USER", defaults::DEFAULT_USER),
];

/// Loads and manages identity files from disk.
pub struct SoulLoader {
    dir: PathBuf,
    identity: RwLock<Identity>,
}

impl SoulLoader {
    /// Create a new SoulLoader, writing default files if the directory is empty.
    pub fn new(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;

        // Write defaults for any missing files
        for (name, default_content) in IDENTITY_FILES {
            let path = dir.join(format!("{name}.md"));
            if !path.exists() {
                std::fs::write(&path, default_content)?;
                info!("Wrote default identity file: {}", path.display());
            }
        }

        let identity = Self::load_from_disk(dir)?;

        Ok(Self {
            dir: dir.to_path_buf(),
            identity: RwLock::new(identity),
        })
    }

    fn load_from_disk(dir: &Path) -> Result<Identity> {
        let mut identity = Identity::new(IdentityMeta::default());

        for (name, default_content) in IDENTITY_FILES {
            let path = dir.join(format!("{name}.md"));
            let content = if path.exists() {
                std::fs::read_to_string(&path)?
            } else {
                (*default_content).to_string()
            };

            let is_default = content == *default_content;

            // Parse YAML frontmatter from IDENTITY.md
            if *name == "IDENTITY"
                && let Some(meta) = parse_identity_frontmatter(&content)
            {
                identity.meta = meta;
            }

            identity.files.insert(
                (*name).to_string(),
                PersonaFile::new(*name, content, is_default),
            );
        }

        Ok(identity)
    }

    /// Get the current identity (read lock).
    pub async fn get(&self) -> Identity {
        self.identity.read().await.clone()
    }

    /// Get a specific identity file by name.
    pub async fn get_file(&self, name: &str) -> Result<PersonaFile> {
        let identity = self.identity.read().await;
        identity
            .files
            .get(name)
            .cloned()
            .ok_or_else(|| MesoError::IdentityNotFound(format!("identity file '{name}' not found")))
    }

    /// Update an identity file's content on disk and in memory.
    pub async fn update_file(&self, name: &str, content: String) -> Result<()> {
        // Verify it's a known file name
        if !IDENTITY_FILES.iter().any(|(n, _)| *n == name) {
            return Err(MesoError::IdentityNotFound(format!(
                "identity file '{name}' not found"
            )));
        }

        let path = self.dir.join(format!("{name}.md"));
        std::fs::write(&path, &content)?;

        let mut identity = self.identity.write().await;
        let default_content = IDENTITY_FILES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, d)| *d)
            .unwrap_or("");
        let is_default = content == default_content;

        identity.files.insert(
            name.to_string(),
            PersonaFile::new(name, content, is_default),
        );

        // Re-parse meta if IDENTITY was updated
        if name == "IDENTITY"
            && let Some(file) = identity.files.get("IDENTITY")
            && let Some(meta) = parse_identity_frontmatter(&file.content)
        {
            identity.meta = meta;
        }

        Ok(())
    }

    /// Reload all files from disk.
    pub async fn reload(&self) -> Result<()> {
        let new_identity = Self::load_from_disk(&self.dir)?;
        let mut identity = self.identity.write().await;
        *identity = new_identity;
        info!("Identity files reloaded from {}", self.dir.display());
        Ok(())
    }

    /// List all identity file names.
    pub async fn list_files(&self) -> Vec<String> {
        let identity = self.identity.read().await;
        let mut names: Vec<String> = identity.files.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Parse YAML frontmatter from an identity file.
/// Expects `---` delimiters around YAML content.
fn parse_identity_frontmatter(content: &str) -> Option<IdentityMeta> {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    let end = after_first.find("---")?;
    let yaml_str = &after_first[..end];

    serde_yaml::from_str(yaml_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn loader_init_writes_defaults() {
        let dir = TempDir::new().unwrap();
        let identity_dir = dir.path().join("identity");
        let _loader = SoulLoader::new(&identity_dir).unwrap();

        assert!(identity_dir.join("SOUL.md").exists());
        assert!(identity_dir.join("IDENTITY.md").exists());
        assert!(identity_dir.join("USER.md").exists());
    }

    #[test]
    fn loader_reads_existing_files() {
        let dir = TempDir::new().unwrap();
        let identity_dir = dir.path().join("identity");
        std::fs::create_dir_all(&identity_dir).unwrap();
        std::fs::write(identity_dir.join("SOUL.md"), "Custom soul content").unwrap();

        let loader = SoulLoader::new(&identity_dir).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let identity = rt.block_on(loader.get());
        let soul = identity.files.get("SOUL").unwrap();
        assert_eq!(soul.content, "Custom soul content");
        assert!(!soul.is_default);
    }

    #[tokio::test]
    async fn loader_get_returns_identity() {
        let dir = TempDir::new().unwrap();
        let loader = SoulLoader::new(&dir.path().join("identity")).unwrap();
        let identity = loader.get().await;
        assert_eq!(identity.meta.name, "MesoClaw");
        assert_eq!(identity.files.len(), 3);
    }

    #[tokio::test]
    async fn loader_get_file_by_name() {
        let dir = TempDir::new().unwrap();
        let loader = SoulLoader::new(&dir.path().join("identity")).unwrap();
        let file = loader.get_file("SOUL").await.unwrap();
        assert_eq!(file.name, "SOUL");
        assert!(file.content.contains("MesoClaw"));
    }

    #[tokio::test]
    async fn loader_get_file_not_found() {
        let dir = TempDir::new().unwrap();
        let loader = SoulLoader::new(&dir.path().join("identity")).unwrap();
        let result = loader.get_file("NONEXISTENT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MesoError::IdentityNotFound(_)
        ));
    }

    #[tokio::test]
    async fn loader_update_file() {
        let dir = TempDir::new().unwrap();
        let identity_dir = dir.path().join("identity");
        let loader = SoulLoader::new(&identity_dir).unwrap();

        loader
            .update_file("SOUL", "Updated soul".into())
            .await
            .unwrap();

        let file = loader.get_file("SOUL").await.unwrap();
        assert_eq!(file.content, "Updated soul");
        assert!(!file.is_default);

        // Verify on disk
        let on_disk = std::fs::read_to_string(identity_dir.join("SOUL.md")).unwrap();
        assert_eq!(on_disk, "Updated soul");
    }

    #[tokio::test]
    async fn loader_reload() {
        let dir = TempDir::new().unwrap();
        let identity_dir = dir.path().join("identity");
        let loader = SoulLoader::new(&identity_dir).unwrap();

        // Modify file on disk directly
        std::fs::write(identity_dir.join("SOUL.md"), "Reloaded content").unwrap();

        loader.reload().await.unwrap();
        let file = loader.get_file("SOUL").await.unwrap();
        assert_eq!(file.content, "Reloaded content");
    }

    #[tokio::test]
    async fn loader_list_files() {
        let dir = TempDir::new().unwrap();
        let loader = SoulLoader::new(&dir.path().join("identity")).unwrap();
        let files = loader.list_files().await;
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"SOUL".to_string()));
        assert!(files.contains(&"IDENTITY".to_string()));
        assert!(files.contains(&"USER".to_string()));
    }

    #[test]
    fn parse_frontmatter_valid() {
        let content = "---\nname: Test\nversion: \"1.0\"\ndescription: A test\n---\n# Body";
        let meta = parse_identity_frontmatter(content).unwrap();
        assert_eq!(meta.name, "Test");
        assert_eq!(meta.version, "1.0");
    }

    #[test]
    fn parse_frontmatter_no_yaml() {
        let content = "# Just markdown\nNo frontmatter here.";
        assert!(parse_identity_frontmatter(content).is_none());
    }
}
