use std::{fs, path::PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use uuid::Uuid;

// ─── Token management ─────────────────────────────────────────────────────────

/// Path to the daemon bearer token file.
pub fn token_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".mesoclaw")
        .join("daemon.token")
}

/// Load the existing token from disk, or generate a new one and persist it.
pub fn load_or_create_token() -> Result<String, String> {
    let path = token_path();

    if path.exists() {
        return fs::read_to_string(&path)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("failed to read token file: {e}"));
    }

    // Generate a new token.
    let token = Uuid::new_v4().to_string().replace('-', "");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create .mesoclaw dir: {e}"))?;
    }

    fs::write(&path, &token).map_err(|e| format!("failed to write token: {e}"))?;

    // Restrict permissions to owner-read-write on Unix.
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&path)
            .map_err(|e| format!("failed to read token metadata: {e}"))?
            .permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)
            .map_err(|e| format!("failed to set token permissions: {e}"))?;
    }

    log::info!("Daemon token written to {}", path.display());
    Ok(token)
}

// ─── Axum middleware ──────────────────────────────────────────────────────────

/// Axum extension: the validated bearer token (injected after middleware passes).
#[derive(Clone)]
pub struct ValidatedToken(pub String);

/// Middleware: validates the `Authorization: Bearer <token>` header.
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let expected = match load_or_create_token() {
        Ok(t) => t,
        Err(e) => {
            log::error!("auth middleware: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let provided = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string);

    match provided {
        Some(token) if token == expected => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
