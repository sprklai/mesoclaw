// Shared path resolution for all file tools.
// Resolves user-provided path strings to absolute OS paths.
// Handles tilde expansion, environment variables, and named directories.

/// Get the home directory path, trying multiple sources.
pub fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
        .or_else(|| {
            directories::UserDirs::new().map(|u| u.home_dir().to_string_lossy().into_owned())
        })
}

/// Resolve a named directory (case-insensitive) to its absolute path.
/// Returns None if the name is not a known directory or the OS does not provide it.
fn resolve_named_dir(name: &str) -> Option<String> {
    let user_dirs = directories::UserDirs::new()?;
    match name.to_lowercase().as_str() {
        "home" => home_dir(),
        "desktop" => user_dirs
            .desktop_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        "downloads" | "download" => user_dirs
            .download_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        "documents" | "document" | "docs" => user_dirs
            .document_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        "pictures" | "photos" => user_dirs
            .picture_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        "videos" | "movies" => user_dirs
            .video_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        "music" => user_dirs
            .audio_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        "templates" => user_dirs
            .template_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        "public" => user_dirs
            .public_dir()
            .map(|p| p.to_string_lossy().into_owned()),
        _ => None,
    }
}

/// Expand environment variables in a string.
///
/// Handles: `${VAR}`, `$VAR` (word-boundary terminated).
/// On Windows, also handles `%VAR%`.
/// Unknown variables are left as-is.
fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'$' && i + 1 < len {
            if bytes[i + 1] == b'{' {
                // ${VAR} form
                if let Some(end) = input[i + 2..].find('}') {
                    let var_name = &input[i + 2..i + 2 + end];
                    if let Ok(val) = std::env::var(var_name) {
                        result.push_str(&val);
                    } else {
                        result.push_str(&input[i..i + 2 + end + 1]);
                    }
                    i += 2 + end + 1;
                } else {
                    result.push('$');
                    i += 1;
                }
            } else {
                // $VAR form — collect alphanumeric + underscore
                let start = i + 1;
                let mut end = start;
                while end < len
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                if end > start {
                    let var_name = &input[start..end];
                    if let Ok(val) = std::env::var(var_name) {
                        result.push_str(&val);
                    } else {
                        result.push_str(&input[i..end]);
                    }
                    i = end;
                } else {
                    result.push('$');
                    i += 1;
                }
            }
        } else if cfg!(windows) && bytes[i] == b'%' {
            // %VAR% form (Windows)
            if let Some(end) = input[i + 1..].find('%') {
                let var_name = &input[i + 1..i + 1 + end];
                if !var_name.is_empty() {
                    if let Ok(val) = std::env::var(var_name) {
                        result.push_str(&val);
                    } else {
                        result.push_str(&input[i..i + 1 + end + 1]);
                    }
                    i += 1 + end + 1;
                } else {
                    result.push('%');
                    i += 1;
                }
            } else {
                result.push('%');
                i += 1;
            }
        } else if let Some(ch) = input[i..].chars().next() {
            result.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    result
}

/// Resolve a user-provided path string to an absolute OS path.
///
/// Resolution order:
/// 1. Absolute paths — passthrough (`/home/...`, `C:\...`)
/// 2. Tilde expansion — `~` → home dir, `~/sub` → home + sub
/// 3. Environment variable expansion — `$VAR`, `${VAR}`, `%VAR%` (Windows)
/// 4. Named directory resolution (case-insensitive):
///    Home, Desktop, Downloads, Documents, Pictures, Videos, Music, Templates, Public
/// 5. Relative paths — returned as-is (fallback)
pub fn resolve_path(raw: &str) -> String {
    let trimmed = raw.trim();

    // 1. Already absolute — pass through
    if trimmed.starts_with('/') || (trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':') {
        return trimmed.to_string();
    }

    // 2. Tilde expansion
    if (trimmed == "~" || trimmed.starts_with("~/"))
        && let Some(home) = home_dir()
    {
        return if trimmed == "~" {
            home
        } else {
            format!("{}{}", home, &trimmed[1..])
        };
    }

    // 3. Environment variable expansion
    let expanded = if trimmed.contains('$') || (cfg!(windows) && trimmed.contains('%')) {
        expand_env_vars(trimmed)
    } else {
        trimmed.to_string()
    };

    // If env expansion produced an absolute path, return it
    if expanded.starts_with('/') || (expanded.len() >= 2 && expanded.as_bytes()[1] == b':') {
        return expanded;
    }

    // 4. Named directory resolution (case-insensitive)
    let (first_segment, rest) = match expanded.find('/') {
        Some(i) => (&expanded[..i], Some(&expanded[i..])),
        None => (expanded.as_str(), None),
    };

    if let Some(base) = resolve_named_dir(first_segment) {
        return match rest {
            Some(suffix) => format!("{base}{suffix}"),
            None => base,
        };
    }

    // 5. Fallback: return as-is (relative path)
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    // P.1 — Absolute unix path passthrough
    #[test]
    fn resolve_absolute_unix() {
        assert_eq!(resolve_path("/absolute/path"), "/absolute/path");
    }

    // P.2 — Absolute windows path passthrough
    #[test]
    fn resolve_absolute_windows() {
        assert_eq!(resolve_path("C:\\Users\\test"), "C:\\Users\\test");
    }

    // P.3 — Tilde only resolves to home
    #[test]
    fn resolve_tilde_only() {
        let resolved = resolve_path("~");
        assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
        assert!(
            !resolved.contains('~'),
            "should not contain tilde: {resolved}"
        );
    }

    // P.4 — Tilde with subpath
    #[test]
    fn resolve_tilde_subpath() {
        let resolved = resolve_path("~/Desktop");
        assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
        assert!(
            resolved.ends_with("/Desktop"),
            "should end with /Desktop: {resolved}"
        );
        assert!(
            !resolved.contains('~'),
            "should not contain tilde: {resolved}"
        );
    }

    // P.5 — Named desktop directory
    #[test]
    fn resolve_named_desktop() {
        let resolved = resolve_path("Desktop");
        if directories::UserDirs::new()
            .and_then(|u| u.desktop_dir().map(|_| ()))
            .is_some()
        {
            assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
            assert!(
                resolved.contains("Desktop") || resolved.contains("desktop"),
                "should resolve Desktop: {resolved}"
            );
        } else {
            assert_eq!(resolved, "Desktop", "fallback to passthrough: {resolved}");
        }
    }

    // P.6 — Named downloads with subpath
    #[test]
    fn resolve_named_downloads_subpath() {
        let resolved = resolve_path("Downloads/file.txt");
        if directories::UserDirs::new()
            .and_then(|u| u.download_dir().map(|_| ()))
            .is_some()
        {
            assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
            assert!(
                resolved.ends_with("/file.txt"),
                "should keep suffix: {resolved}"
            );
        } else {
            assert_eq!(
                resolved, "Downloads/file.txt",
                "fallback to passthrough: {resolved}"
            );
        }
    }

    // P.7 — Named pictures directory
    #[test]
    fn resolve_named_pictures() {
        let resolved = resolve_path("Pictures");
        if directories::UserDirs::new()
            .and_then(|u| u.picture_dir().map(|_| ()))
            .is_some()
        {
            assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
        } else {
            assert_eq!(resolved, "Pictures");
        }
    }

    // P.8 — Named music directory
    #[test]
    fn resolve_named_music() {
        let resolved = resolve_path("Music");
        if directories::UserDirs::new()
            .and_then(|u| u.audio_dir().map(|_| ()))
            .is_some()
        {
            assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
        } else {
            assert_eq!(resolved, "Music");
        }
    }

    // P.9 — Named directories are case-insensitive
    #[test]
    fn resolve_named_case_insensitive() {
        let upper = resolve_path("DESKTOP");
        let lower = resolve_path("desktop");
        assert_eq!(upper, lower);
    }

    // P.10 — Environment variable with $VAR
    #[test]
    fn resolve_env_var_dollar() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let resolved = resolve_path("$HOME/test.txt");
        assert_eq!(resolved, format!("{home}/test.txt"));
    }

    // P.11 — Environment variable with ${VAR}
    #[test]
    fn resolve_env_var_braced() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let resolved = resolve_path("${HOME}/test.txt");
        assert_eq!(resolved, format!("{home}/test.txt"));
    }

    // P.12 — Unknown relative path returned as-is
    #[test]
    fn resolve_unknown_relative() {
        assert_eq!(resolve_path("unknown_relative"), "unknown_relative");
    }
}
