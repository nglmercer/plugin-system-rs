//! Bearer-token authentication for the HTTP and WebSocket API.
//!
//! This daemon drives OBS, injects keystrokes, fetches URLs server-side and
//! loads uploaded code. Every one of those is reachable over the same socket,
//! so "anyone who can open a TCP connection" is not an acceptable authorization
//! rule — not even on a laptop, where any page in any browser can POST to
//! `localhost`.
//!
//! The scheme is deliberately the simplest thing that works for a single-user
//! local daemon: one long random token, generated on first run and stored in
//! the user's data directory. Presenting it is proof of being the local user,
//! since only the local user can read that file.
//!
//! Callers present it as `Authorization: Bearer <token>`, `X-SD-Token: <token>`
//! or `?token=<token>`. The query form exists because a browser `WebSocket`
//! cannot set request headers; it is otherwise the weakest of the three (URLs
//! land in logs), so prefer a header where you have the choice.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::{response::ApiResponse, state::AppState};

/// Query/header names a token may arrive under.
const TOKEN_HEADER: &str = "x-sd-token";
const TOKEN_QUERY_KEY: &str = "token";

/// The API token for this installation.
#[derive(Clone)]
pub struct ApiAuth {
    token: Arc<String>,
}

impl ApiAuth {
    /// Load the token from `SD_API_TOKEN`, or from the on-disk token file,
    /// generating and persisting one on first run.
    ///
    /// Never fails: a daemon that cannot persist a token still needs *a*
    /// token, and an ephemeral one (regenerated next start) is far better than
    /// falling back to no authentication at all.
    pub fn load_or_create() -> Self {
        if let Ok(token) = std::env::var("SD_API_TOKEN") {
            if !token.is_empty() {
                return Self {
                    token: Arc::new(token),
                };
            }
        }

        let path = token_path();
        if let Ok(existing) = std::fs::read_to_string(&path) {
            let existing = existing.trim().to_string();
            if !existing.is_empty() {
                return Self {
                    token: Arc::new(existing),
                };
            }
        }

        let token = generate_token();
        if let Err(e) = persist_token(&path, &token) {
            log::warn!(
                "could not write the API token to {}: {e}. A new token will be \
                 generated on the next start.",
                path.display()
            );
        }
        Self {
            token: Arc::new(token),
        }
    }

    /// Build from a known token. For tests and for embedders that manage
    /// their own secret.
    pub fn from_token(token: impl Into<String>) -> Self {
        Self {
            token: Arc::new(token.into()),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Compare in constant time.
    ///
    /// A byte-by-byte `==` leaks the length of the matching prefix through
    /// timing, which is enough to recover a token one character at a time
    /// against a local service that answers fast and often.
    pub fn matches(&self, presented: &str) -> bool {
        constant_time_eq(self.token.as_bytes(), presented.as_bytes())
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Where the token lives when the environment does not supply one.
pub fn token_path() -> PathBuf {
    if let Ok(path) = std::env::var("SD_API_TOKEN_FILE") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    sd_paths::mutable_data_dir().join("api-token")
}

/// 256 bits of randomness, hex-encoded.
///
/// Sourced from two v4 UUIDs rather than a new dependency: their random bits
/// come from the OS CSPRNG, which is the property that matters here.
fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn persist_token(path: &PathBuf, token: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, token)?;

    // The token *is* the credential, so it must not be world-readable on a
    // multi-user box. Windows has no equivalent one-liner; the per-user data
    // directory is already ACL'd there.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Pull a token out of a request, whichever of the three forms it used.
fn presented_token<B>(req: &Request<B>) -> Option<String> {
    if let Some(value) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(value) = value.to_str() {
            if let Some(rest) = value.strip_prefix("Bearer ").or(value.strip_prefix("bearer ")) {
                return Some(rest.trim().to_string());
            }
        }
    }

    if let Some(value) = req.headers().get(TOKEN_HEADER) {
        if let Ok(value) = value.to_str() {
            return Some(value.trim().to_string());
        }
    }

    let query = req.uri().query()?;
    form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == TOKEN_QUERY_KEY)
        .map(|(_, value)| value.into_owned())
}

/// Reject any request that does not present the API token.
pub async fn require_token(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    match presented_token(&req) {
        Some(token) if state.auth.matches(&token) => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error(
                "Missing or invalid API token. Pass it as an Authorization: Bearer \
                 header, an X-SD-Token header, or a ?token= query parameter."
                    .to_string(),
            )),
        )
            .into_response(),
    }
}

#[derive(serde::Serialize)]
pub(crate) struct TokenResponse {
    pub token: String,
}

/// Hand the token to a caller that is already on the machine.
///
/// This is the bootstrap: the dashboard is served from the same origin and has
/// no other way to learn the secret. Answering only on loopback keeps it a
/// local-user convenience rather than a way for the whole LAN to help itself —
/// anyone who can reach this over 127.0.0.1 can read the token file anyway.
pub(crate) async fn get_token(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
) -> Response {
    if !peer.ip().is_loopback() {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error(
                "The API token is only handed out to local clients. Copy it from the \
                 token file or the server's startup output."
                    .to_string(),
            )),
        )
            .into_response();
    }

    Json(ApiResponse::success(TokenResponse {
        token: state.auth.token().to_string(),
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_still_compares_correctly() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn generated_tokens_are_long_and_distinct() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 64, "256 bits, hex encoded");
        assert_ne!(a, b);
    }

    #[test]
    fn accepts_every_documented_token_form() {
        let build = |f: fn(axum::http::request::Builder) -> axum::http::request::Builder| {
            f(Request::builder().uri("/api/devices"))
                .body(())
                .unwrap()
        };

        let bearer = build(|b| b.header("authorization", "Bearer secret"));
        assert_eq!(presented_token(&bearer).as_deref(), Some("secret"));

        let custom = build(|b| b.header(TOKEN_HEADER, "secret"));
        assert_eq!(presented_token(&custom).as_deref(), Some("secret"));

        // The WebSocket form: a browser cannot set headers on the handshake.
        let query = Request::builder()
            .uri("/ws?token=secret")
            .body(())
            .unwrap();
        assert_eq!(presented_token(&query).as_deref(), Some("secret"));

        let none = build(|b| b);
        assert!(presented_token(&none).is_none());
    }

    #[test]
    fn a_token_query_value_is_url_decoded() {
        let req = Request::builder()
            .uri("/ws?other=1&token=a%2Bb")
            .body(())
            .unwrap();
        assert_eq!(presented_token(&req).as_deref(), Some("a+b"));
    }

    #[test]
    fn matches_rejects_a_near_miss() {
        let auth = ApiAuth::from_token("correct-horse");
        assert!(auth.matches("correct-horse"));
        assert!(!auth.matches("correct-hors"));
        assert!(!auth.matches("Correct-horse"));
    }
}
