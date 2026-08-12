//! The obs-websocket 5.x wire protocol.
//!
//! Kept guest-side deliberately. The host grants a plain socket; everything
//! from the identify handshake to request correlation happens in here, which
//! is what makes this plugin more than a forwarding shim — and means a
//! compromised OBS server is talking to sandboxed code, not to the host.
//!
//! Reference: <https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md>

use base64::Engine as _;
use sha2::{Digest, Sha256};

/// Opcodes, as defined by the protocol.
pub mod op {
    /// Server → client, first message after connect.
    pub const HELLO: u64 = 0;
    /// Client → server, authentication response.
    pub const IDENTIFY: u64 = 1;
    /// Server → client, session established.
    pub const IDENTIFIED: u64 = 2;
    /// Client → server, a request.
    pub const REQUEST: u64 = 6;
    /// Server → client, the matching response.
    pub const REQUEST_RESPONSE: u64 = 7;
    /// Server → client, an unsolicited event.
    pub const EVENT: u64 = 5;
}

/// Which protocol features to subscribe to.
///
/// Zero: this plugin polls rather than subscribing. Events would arrive
/// interleaved with responses and there is no background reader to drain
/// them — a request/response model is a better fit for a plugin whose calls
/// are driven by HTTP requests anyway.
const EVENT_SUBSCRIPTIONS: u64 = 0;

/// The RPC version this client implements.
const RPC_VERSION: u64 = 1;

/// Compute the authentication string for a `Hello` that requires it.
///
/// The scheme is
/// `base64(sha256(base64(sha256(password + salt)) + challenge))`.
pub fn auth_response(password: &str, salt: &str, challenge: &str) -> String {
    let engine = base64::engine::general_purpose::STANDARD;

    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    let secret = engine.encode(hasher.finalize());

    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(challenge.as_bytes());
    engine.encode(hasher.finalize())
}

/// Build the `Identify` payload to answer a `Hello`.
///
/// `hello_d` is the `d` object of the received Hello. Authentication is only
/// included when the server asked for it; sending it unprompted is an error
/// on some builds.
pub fn identify_message(hello_d: &serde_json::Value, password: &str) -> serde_json::Value {
    let mut d = serde_json::json!({
        "rpcVersion": RPC_VERSION,
        "eventSubscriptions": EVENT_SUBSCRIPTIONS,
    });

    if let Some(auth) = hello_d.get("authentication") {
        let salt = auth.get("salt").and_then(|v| v.as_str()).unwrap_or_default();
        let challenge = auth
            .get("challenge")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        d["authentication"] = serde_json::Value::String(auth_response(password, salt, challenge));
    }

    serde_json::json!({ "op": op::IDENTIFY, "d": d })
}

/// Build a request frame.
pub fn request_message(
    request_type: &str,
    request_id: &str,
    data: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut d = serde_json::json!({
        "requestType": request_type,
        "requestId": request_id,
    });
    if let Some(data) = data {
        d["requestData"] = data;
    }
    serde_json::json!({ "op": op::REQUEST, "d": d })
}

/// The outcome of a request, once its response frame has been matched.
pub enum Response {
    Ok(serde_json::Value),
    Failed(String),
}

/// Interpret a `RequestResponse` frame's `d` object.
pub fn parse_response(d: &serde_json::Value) -> Response {
    let status = d.get("requestStatus");
    let ok = status
        .and_then(|s| s.get("result"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if ok {
        // A request with no return data omits `responseData` entirely; an
        // empty object keeps callers from having to special-case that.
        Response::Ok(
            d.get("responseData")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        )
    } else {
        let comment = status
            .and_then(|s| s.get("comment"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let code = status
            .and_then(|s| s.get("code"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        Response::Failed(comment.unwrap_or_else(|| format!("request failed with code {code}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected values cross-checked against an independent implementation of
    /// the documented algorithm
    /// (`b64(sha256(b64(sha256(password+salt)) + challenge))`), not copied
    /// from a published vector. They pin the algorithm against accidental
    /// change — a hashing order swap here would fail every OBS login.
    #[test]
    fn auth_response_matches_an_independent_implementation() {
        assert_eq!(
            auth_response(
                "supersecretpassword",
                "PZVbYpvAnZut2SS6JNJytDm9",
                "ztTBnnuqrqaKDzRM3xcVdbYm"
            ),
            "zZgWipvwSGrw748kHN4gNpBC1IaeiiWX3Hjkrm849Sc="
        );
        assert_eq!(
            auth_response("password", "salt", "challenge"),
            "zTM5ki6L2vVvBQiTG9ckH1Lh64AbnCf6XZ226UmnkIA="
        );
    }

    /// Salt and challenge are hashed at different stages, so swapping them
    /// must change the result. Guards the ordering the test above pins.
    #[test]
    fn salt_and_challenge_are_not_interchangeable() {
        assert_ne!(
            auth_response("pw", "aaa", "bbb"),
            auth_response("pw", "bbb", "aaa")
        );
    }

    #[test]
    fn identify_omits_auth_when_the_server_did_not_ask() {
        let hello = serde_json::json!({ "rpcVersion": 1 });
        let msg = identify_message(&hello, "unused");
        assert_eq!(msg["op"], op::IDENTIFY);
        assert!(
            msg["d"].get("authentication").is_none(),
            "auth must not be sent unprompted: {msg}"
        );
    }

    #[test]
    fn identify_includes_auth_when_challenged() {
        let hello = serde_json::json!({
            "rpcVersion": 1,
            "authentication": { "salt": "s", "challenge": "c" }
        });
        let msg = identify_message(&hello, "pw");
        assert_eq!(msg["d"]["authentication"], auth_response("pw", "s", "c"));
    }

    #[test]
    fn a_successful_response_yields_its_data() {
        let d = serde_json::json!({
            "requestStatus": { "result": true, "code": 100 },
            "responseData": { "currentProgramSceneName": "Scene 1" }
        });
        match parse_response(&d) {
            Response::Ok(v) => assert_eq!(v["currentProgramSceneName"], "Scene 1"),
            Response::Failed(e) => panic!("expected success, got: {e}"),
        }
    }

    #[test]
    fn a_response_without_data_is_still_a_success() {
        let d = serde_json::json!({ "requestStatus": { "result": true, "code": 100 } });
        assert!(matches!(parse_response(&d), Response::Ok(_)));
    }

    #[test]
    fn a_failed_response_surfaces_the_comment() {
        let d = serde_json::json!({
            "requestStatus": { "result": false, "code": 600, "comment": "No such scene" }
        });
        match parse_response(&d) {
            Response::Ok(v) => panic!("expected failure, got: {v}"),
            Response::Failed(e) => assert_eq!(e, "No such scene"),
        }
    }

    /// A failure with no comment must still say something useful.
    #[test]
    fn a_failed_response_without_a_comment_reports_its_code() {
        let d = serde_json::json!({ "requestStatus": { "result": false, "code": 604 } });
        match parse_response(&d) {
            Response::Ok(v) => panic!("expected failure, got: {v}"),
            Response::Failed(e) => assert!(e.contains("604"), "got: {e}"),
        }
    }
}
