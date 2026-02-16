use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryContents {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<FileEntry>,
}

/// List contents of a directory, optionally filtering by file extensions
#[tauri::command]
pub async fn list_directory(
    path: Option<String>,
    extensions: Option<Vec<String>>,
) -> Result<DirectoryContents, String> {
    let dir_path = match path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => dirs::home_dir().ok_or("Could not determine home directory")?,
    };

    if !dir_path.exists() {
        return Err(format!("Directory does not exist: {}", dir_path.display()));
    }

    if !dir_path.is_dir() {
        return Err(format!("Path is not a directory: {}", dir_path.display()));
    }

    let parent = dir_path.parent().map(|p| p.to_string_lossy().to_string());

    let mut entries = Vec::new();

    let read_dir =
        std::fs::read_dir(&dir_path).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in read_dir.flatten() {
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files/directories (starting with .)
        if name.starts_with('.') {
            continue;
        }

        let is_directory = metadata.is_dir();
        let path_str = entry.path().to_string_lossy().to_string();

        // Filter files by extension if specified
        if !is_directory {
            if let Some(ref exts) = extensions {
                let has_matching_ext = exts.iter().any(|ext| {
                    name.to_lowercase()
                        .ends_with(&format!(".{}", ext.to_lowercase()))
                });
                if !has_matching_ext {
                    continue;
                }
            }
        }

        let size = if is_directory {
            None
        } else {
            Some(metadata.len())
        };

        let modified = metadata.modified().ok().map(|time| {
            let datetime: chrono::DateTime<chrono::Local> = time.into();
            datetime.format("%Y-%m-%d %H:%M").to_string()
        });

        entries.push(FileEntry {
            name,
            path: path_str,
            is_directory,
            size,
            modified,
        });
    }

    // Sort: directories first, then files, both alphabetically
    entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(DirectoryContents {
        path: dir_path.to_string_lossy().to_string(),
        parent,
        entries,
    })
}

/// Get the user's home directory path
#[tauri::command]
pub fn get_home_directory() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".to_string())
}

/// Get common directories (home, documents, downloads, desktop)
#[tauri::command]
pub fn get_common_directories() -> Vec<(String, String)> {
    let mut dirs_list = Vec::new();

    if let Some(home) = dirs::home_dir() {
        dirs_list.push(("Home".to_string(), home.to_string_lossy().to_string()));
    }

    if let Some(documents) = dirs::document_dir() {
        dirs_list.push((
            "Documents".to_string(),
            documents.to_string_lossy().to_string(),
        ));
    }

    if let Some(downloads) = dirs::download_dir() {
        dirs_list.push((
            "Downloads".to_string(),
            downloads.to_string_lossy().to_string(),
        ));
    }

    if let Some(desktop) = dirs::desktop_dir() {
        dirs_list.push(("Desktop".to_string(), desktop.to_string_lossy().to_string()));
    }

    dirs_list
}
