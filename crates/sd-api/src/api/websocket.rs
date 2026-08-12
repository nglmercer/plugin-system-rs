use axum::{
    extract::ws::Message,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use sd_types::DeviceId;

use crate::state::AppState;

pub(crate) async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

pub(crate) async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    let devices = state.device_manager.list_devices().await;
    let initial_state = serde_json::json!({
        "type": "initial_state",
        "devices": devices,
    });
    let _ = sender.send(Message::Text(initial_state.to_string())).await;

    // Subscribe to the broadcast channel rather than registering a callback:
    // a `subscribe_all` callback is never unregistered (one leaked per
    // connection) and only fires while `EventBus::run` is alive, so a stalled
    // dispatch loop silently froze every client. A receiver drops itself with
    // the connection and is fed by the channel directly.
    let mut rx = state.events.subscribe_channel();

    let send_task = tokio::spawn(async move {
        loop {
            let event = match rx.recv().await {
                Ok(event) => event,
                // The client fell behind its 1024-event buffer. Say so instead
                // of dropping events invisibly, then keep serving.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("websocket client lagged; {skipped} event(s) dropped");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };

            let msg = serde_json::json!({
                "type": "event",
                "event": event,
            });

            if sender.send(Message::Text(msg.to_string())).await.is_err() {
                break;
            }
        }
    });

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                        if msg_type == "press_button" {
                            if let (Some(device), Some(index)) = (
                                parsed.get("device").and_then(|d| d.as_str()),
                                parsed.get("index").and_then(|i| i.as_u64()),
                            ) {
                                if let Some(dev) = state
                                    .device_manager
                                    .get_device(&DeviceId(device.to_string()))
                                    .await
                                {
                                    dev.press_button(index as usize);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_frame)) => {
                break;
            }
            _ => {}
        }
    }

    send_task.abort();
}
