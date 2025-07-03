use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path,
        Query,
    },
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse},
    routing::{any, get},
    Router,
};
use tokio::net::TcpListener;
use serde::Deserialize;
use axum_extra::{headers::Authorization, TypedHeader};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum::extract::State;
use std::sync::Arc;
use dotenvy::dotenv;
use std::env;
use base64::engine::general_purpose;
use base64::Engine;
use dashmap::DashMap;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct ClientParams {
    client_id: String,
}

#[derive(Clone)]
struct AppState {
    secret_token: String,
    active_websockets: Arc<DashMap<String, tokio::sync::mpsc::Sender<Message>>>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct TunneledRequest {
    id: String,
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String, // base64 encoded
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let secret_token = env::var("SECRET_TOKEN")
        .expect("SECRET_TOKEN must be set in .env file or environment");

    let app_state = Arc::new(AppState {
        secret_token,
        active_websockets: Arc::new(DashMap::new()),
    });

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_tunnel_server=debug,tower_http=debug,reqwest=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Simplified Rust Tunnel Server...");

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/:client_id", any(forward_handler_no_path))
        .route("/:client_id/*path", any(forward_handler_with_path))
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}


async fn handle_forwarding_request(
    app_state: Arc<AppState>,
    client_id: String,
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
    forward_path: String,
) -> impl IntoResponse {
    info!(
        "Forwarding request for client_id: {}, path: {}, method: {}",
        client_id,
        forward_path,
        method.as_str()
    );

    if let Some(ws_sender) = app_state.active_websockets.get(&client_id) {
        let headers_map: HashMap<String, String> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();

        let tunneled_request = TunneledRequest {
            id: Uuid::new_v4().to_string(),
            method: method.to_string(),
            path: forward_path,
            headers: headers_map,
            body: general_purpose::STANDARD.encode(body),
        };

        match serde_json::to_string(&tunneled_request) {
            Ok(json_payload) => {
                if let Err(e) = ws_sender.send(Message::Text(json_payload)).await {
                    error!("Failed to send request to websocket: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to forward request to client",
                    )
                        .into_response();
                }
                (StatusCode::OK, "Request forwarded").into_response()
            }
            Err(e) => {
                error!("Failed to serialize request: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to serialize request",
                )
                    .into_response()
            }
        }
    } else {
        (StatusCode::NOT_FOUND, "Client not connected").into_response()
    }
}

#[axum::debug_handler]
async fn forward_handler_no_path(
    State(app_state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    let forward_path = "/".to_string();
    handle_forwarding_request(app_state, client_id, method, headers, body, forward_path).await
}

#[axum::debug_handler]
async fn forward_handler_with_path( // Renamed for clarity
    State(app_state): State<Arc<AppState>>,
    Path((client_id, path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    let forward_path = format!("/{}", path);
    handle_forwarding_request(app_state, client_id, method, headers, body, forward_path).await
}


#[axum::debug_handler]
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ClientParams>,
    auth_header: Option<TypedHeader<Authorization<axum_extra::headers::authorization::Bearer>>>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Attempting to upgrade connection to WebSocket on /ws");
    let auth_header: Authorization<axum_extra::headers::authorization::Bearer> =
        if let Some(TypedHeader(auth_header)) = auth_header {
            auth_header
        } else {
            error!("Missing Authorization header");
            return (axum::http::StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response();
        };

    if auth_header.token() != app_state.secret_token {
        error!("Invalid token provided: {}", auth_header.token());
        return (StatusCode::FORBIDDEN, "Invalid token").into_response();
    }

    info!(
        "WebSocket connection authorized for client_id: {}",
        params.client_id
    );

    let client_id = params.client_id.clone();
    ws.on_upgrade(move |socket| {
        handle_single_websocket(socket, app_state, client_id)
    })
}

async fn handle_single_websocket(
    mut socket: WebSocket,
    app_state: Arc<AppState>,
    client_id: String,
) {
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
                        // Here you would handle responses from the client
                    }
                    Message::Binary(bin) => {
                        info!("Received binary from WebSocket: {:?}", bin);
                        // Here you would handle responses from the client
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
}