//! Server-side fetch for the `fetch` widget.
//!
//! This endpoint issues requests *from the host*, so an unguarded version is a
//! textbook SSRF: the caller picks the URL and the daemon reaches whatever the
//! machine can reach — cloud metadata at `169.254.169.254`, admin ports bound
//! to loopback, anything on the LAN. Everything below exists to stop that.
//!
//! The policy is deny-by-default for non-public destinations. Someone whose
//! deck genuinely needs to poke a device on their own network sets
//! `SD_PROXY_ALLOW_PRIVATE=1` and takes that risk knowingly.

use crate::response::ApiResponse;
use axum::{response::IntoResponse, Json};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Deserialize)]
pub struct ProxyRequest {
    pub url: String,
    pub method: Option<String>,
    pub body: Option<serde_json::Value>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Serialize)]
pub struct ProxyResponse {
    pub status: u16,
    pub body: serde_json::Value,
    pub headers: std::collections::HashMap<String, String>,
}

use crate::state::AppState;
use axum::extract::State;

/// Ceiling on a proxied response body. The whole thing is buffered into a
/// JSON envelope, so without a cap a single request can exhaust host memory.
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

/// Headers a caller may not set. `host` would let a request pass the address
/// check against one name and be routed by the server as another; the rest are
/// hop-by-hop headers that are not the caller's to control.
const FORBIDDEN_REQUEST_HEADERS: &[&str] = &[
    "host",
    "connection",
    "proxy-authorization",
    "transfer-encoding",
    "upgrade",
];

/// Build the client the proxy uses.
///
/// Redirects are **off**. A redirect is a second request to an address the
/// caller never showed us, which is the standard way to walk an allowlist
/// check: fetch a public URL that 302s to `http://169.254.169.254/`. With the
/// policy set to none, the 3xx is returned to the caller as a result and any
/// follow-up is a fresh, separately validated call.
pub fn proxy_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Whether the operator opted out of the private-address block.
fn private_targets_allowed() -> bool {
    std::env::var("SD_PROXY_ALLOW_PRIVATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Addresses that must never be reached on a caller's behalf.
///
/// Deliberately a denylist of "not publicly routable" rather than an allowlist
/// of one or two ranges: the interesting targets (metadata services, loopback
/// admin panels, LAN devices, CGNAT space) are spread across many blocks, and
/// each one omitted is a hole.
fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        // An IPv4-mapped address is an IPv4 destination wearing a v6 name, so
        // check it as one — otherwise `::ffff:127.0.0.1` walks straight past.
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_blocked_ipv4(v4),
            None => is_blocked_ipv6(v6),
        },
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        // 100.64.0.0/10, carrier-grade NAT.
        || (a == 100 && (64..128).contains(&b))
        // 192.0.0.0/24, IETF protocol assignments.
        || ip.octets()[..3] == [192, 0, 0]
        // 198.18.0.0/15, benchmarking.
        || (a == 198 && (b == 18 || b == 19))
        // 240.0.0.0/4, reserved.
        || a >= 240
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    let first = ip.segments()[0];
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // fc00::/7, unique local.
        || (first & 0xfe00) == 0xfc00
        // fe80::/10, link local.
        || (first & 0xffc0) == 0xfe80
        // 2001:db8::/32, documentation.
        || (first == 0x2001 && ip.segments()[1] == 0x0db8)
}

/// Reject a URL the proxy must not fetch.
///
/// Resolution happens here so a hostname pointing at a private address is
/// caught too. This is not proof against DNS rebinding — the name could
/// resolve differently when reqwest connects — but it closes the direct case,
/// and the redirect policy closes the other common one.
async fn validate_target(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "Unsupported URL scheme '{other}': the proxy only fetches http and https"
            ))
        }
    }

    if private_targets_allowed() {
        return Ok(());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;
    let port = parsed.port_or_known_default().unwrap_or(80);

    // A literal address needs no lookup, and must not get one: resolving it
    // would be a no-op that only adds a failure mode.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return reject_if_blocked(ip);
    }

    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("Could not resolve '{host}': {e}"))?;

    let mut saw_any = false;
    for addr in addrs {
        saw_any = true;
        reject_if_blocked(addr.ip())?;
    }

    if !saw_any {
        return Err(format!("Could not resolve '{host}'"));
    }

    Ok(())
}

fn reject_if_blocked(ip: IpAddr) -> Result<(), String> {
    if is_blocked_ip(ip) {
        return Err(format!(
            "Refusing to proxy to {ip}: the address is loopback, private, or otherwise \
             not publicly routable. Set SD_PROXY_ALLOW_PRIVATE=1 to allow it."
        ));
    }
    Ok(())
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    Json(req): Json<ProxyRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_target(&req.url).await {
        return Json(ApiResponse::<ProxyResponse>::error(e));
    }

    let client = &state.http_client;
    let method = match req.method.as_deref() {
        Some("POST") => reqwest::Method::POST,
        Some("PUT") => reqwest::Method::PUT,
        Some("DELETE") => reqwest::Method::DELETE,
        _ => reqwest::Method::GET,
    };

    let mut rb = client.request(method, &req.url);

    if let Some(headers) = req.headers {
        for (k, v) in headers {
            if FORBIDDEN_REQUEST_HEADERS
                .iter()
                .any(|forbidden| k.eq_ignore_ascii_case(forbidden))
            {
                continue;
            }
            rb = rb.header(k, v);
        }
    }

    if let Some(body) = req.body {
        rb = rb.json(&body);
    }

    match rb.send().await {
        Ok(res) => {
            let status = res.status().as_u16();
            let mut headers = std::collections::HashMap::new();
            for (name, value) in res.headers() {
                if let Ok(value_str) = value.to_str() {
                    headers.insert(name.to_string(), value_str.to_string());
                }
            }

            let is_json = res
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("application/json"))
                .unwrap_or(false);

            let bytes = match read_capped_body(res).await {
                Ok(bytes) => bytes,
                Err(e) => return Json(ApiResponse::error(e)),
            };

            let body = if is_json {
                serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
            };

            Json(ApiResponse::success(ProxyResponse {
                status,
                body,
                headers,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

/// Read a response body, refusing anything over [`MAX_RESPONSE_BYTES`].
///
/// Streamed rather than `res.bytes()` so an oversized (or endless) body is
/// abandoned partway instead of being buffered in full before the check.
async fn read_capped_body(res: reqwest::Response) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Failed to read response body: {e}"))?;
        if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(format!(
                "Response body exceeds the {MAX_RESPONSE_BYTES} byte proxy limit"
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_the_addresses_that_matter() {
        for raw in [
            "127.0.0.1",
            "0.0.0.0",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            // The cloud metadata endpoint, the single most valuable SSRF target.
            "169.254.169.254",
            "100.64.0.1",
            "255.255.255.255",
            "::1",
            "fe80::1",
            "fd00::1",
            // An IPv4 loopback smuggled in as IPv6.
            "::ffff:127.0.0.1",
        ] {
            let ip: IpAddr = raw.parse().unwrap();
            assert!(is_blocked_ip(ip), "{raw} should be blocked");
        }
    }

    #[test]
    fn allows_public_addresses() {
        for raw in ["1.1.1.1", "93.184.216.34", "2606:4700:4700::1111"] {
            let ip: IpAddr = raw.parse().unwrap();
            assert!(!is_blocked_ip(ip), "{raw} should be allowed");
        }
    }

    #[tokio::test]
    async fn rejects_non_http_schemes() {
        // `file://` would read the host's disk; `gopher://` is the classic
        // protocol-smuggling vector.
        for raw in ["file:///etc/passwd", "gopher://127.0.0.1:6379/_FLUSHALL"] {
            let err = validate_target(raw).await.unwrap_err();
            assert!(
                err.contains("scheme"),
                "{raw} should be refused for its scheme, got: {err}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_literal_private_targets() {
        let err = validate_target("http://169.254.169.254/latest/meta-data/")
            .await
            .unwrap_err();
        assert!(err.contains("169.254.169.254"), "got: {err}");
    }
}
