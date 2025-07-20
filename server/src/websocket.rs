use crate::models::ClientParams;
use crate::models::TunneledHttpResponse;
use crate::AppState;

use crate::access_control;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use axum_extra::{headers::Authorization, TypedHeader};
use std::sync::Arc;
use tracing::{error, info};

#[axum::debug_handler]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ClientParams>,
    auth_header: Option<TypedHeader<Authorization<axum_extra::headers::authorization::Bearer>>>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Attempting to upgrade connection to WebSocket on /ws");

    if let Err(e) = access_control::authenticate_client(auth_header, &params, &app_state) {
        error!("Authentication failed");
        return e.into_response();
    }

    let client_id = params.client_id.clone();
    let allowed_paths = params.allowed_paths.clone();
    if let Err(e) = access_control::add_allowed_paths(&app_state, &client_id, allowed_paths) {
        error!("Failed to add allowed paths");
        return e.into_response();
    }

    let allowed_ips = params.allowed_ips.clone();
    if let Err(e) = access_control::add_allowed_ips(&app_state, &client_id, allowed_ips) {
        error!("Failed to add allowed IPs");
        return e.into_response();
    }

    let allowed_asns = params.allowed_asns.clone();
    if let Err(e) = access_control::add_allowed_asns(&app_state, &client_id, allowed_asns) {
        error!("Failed to add allowed ASNs");
        return e.into_response();
    }

    ws.on_upgrade(move |socket| handle_websocket(socket, app_state, client_id))
}

async fn handle_websocket(mut socket: WebSocket, app_state: Arc<AppState>, client_id: String) {
    info!("WebSocket connected for client_id: {}", client_id);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(100);
    app_state.active_websockets.insert(client_id.clone(), tx);

    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                if socket.send(msg).await.is_err() {
                    error!("Failed to send message to websocket");
                    break;
                }
            }
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Text(text) => {
                        info!("Received text from WebSocket: {}", text);
                        if let Ok(response) = serde_json::from_str::<TunneledHttpResponse>(&text) {
                            if let Some((_, tx)) = app_state.pending_responses.remove(&response.id) {
                                if tx.send(response).is_err() {
                                    error!("Failed to send response to pending request");
                                }
                            }
                        }
                    }
                    Message::Binary(bin) => {
                        info!("Received binary from WebSocket: {:?}" , bin);
                    }
                    Message::Ping(ping) => {
                        info!("Received Ping from WebSocket. Sending Pong.");
                        if let Err(e) = socket.send(Message::Pong(ping)).await {
                            error!("Failed to send Pong to WebSocket: {:?}", e);
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        info!("Received Pong from WebSocket.");
                    }
                    Message::Close(close_frame) => {
                        info!("Received Close from WebSocket: {:?}", close_frame);
                        break;
                    }
                }
            }
            else => {
                break;
            }
        }
    }

    info!("WebSocket for client_id: {} disconnected.", client_id);
    app_state.active_websockets.remove(&client_id);
    app_state.allowed_paths.remove(&client_id);
    app_state.allowed_ips.remove(&client_id);
}
