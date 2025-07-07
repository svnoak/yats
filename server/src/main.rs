use axum::{
    extract::ws::Message,
    routing::{any, get},
    Router,
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::info;

use crate::models::TunneledHttpResponse;

mod access_control;
mod config;
mod forwarding;
mod logging;
mod models;
mod websocket;

#[derive(Clone)]
pub struct AppState {
    pub is_production: bool,
    pub secret_token: String,
    pub active_websockets: Arc<DashMap<String, tokio::sync::mpsc::Sender<Message>>>,
    pub pending_responses: Arc<DashMap<String, oneshot::Sender<TunneledHttpResponse>>>,
    pub allowed_paths: Arc<DashMap<String, Vec<String>>>,
}

impl AppState {
    pub fn new(config: config::Config) -> Self {
        Self {
            is_production: config.is_production,
            secret_token: config.secret_token,
            active_websockets: Arc::new(DashMap::new()),
            pending_responses: Arc::new(DashMap::new()),
            allowed_paths: Arc::new(DashMap::new()),
        }
    }
}

#[tokio::main]
async fn main() {
    let config = config::Config::new();
    let app_state = Arc::new(AppState::new(config));
    logging::setup_tracing();

    info!("Starting Simplified Rust Tunnel Server...");

    let app = Router::new()
        .route("/ws", get(websocket::ws_handler))
        .route("/:client_id", any(forwarding::forward_handler_no_path))
        .route(
            "/:client_id/*path",
            any(forwarding::forward_handler_with_path),
        )
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
