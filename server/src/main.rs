use axum::{
    extract::ws::Message,
    routing::{any, get},
    Router,
};
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
use tracing::info;

use crate::models::TunneledHttpResponse;

mod access_control;
mod asn_updater;
mod config;
mod forwarding;
mod logging;
mod models;
mod websocket;

#[derive(Clone)]
pub struct AppState {
    pub maxmind_license_key: String,
    pub is_production: bool,
    pub secret_token: String,
    pub active_websockets: Arc<DashMap<String, tokio::sync::mpsc::Sender<Message>>>,
    pub pending_responses: Arc<DashMap<String, oneshot::Sender<TunneledHttpResponse>>>,
    pub allowed_paths: Arc<DashMap<String, Vec<String>>>,
    pub allowed_ips: Arc<DashMap<String, Vec<String>>>,
    pub allowed_asns: Arc<DashMap<String, Vec<u32>>>,
    pub db_reader: Arc<RwLock<maxminddb::Reader<Vec<u8>>>>,
}

impl AppState {
    pub fn new(config: config::Config) -> Self {
        Self {
            is_production: config.is_production,
            secret_token: config.secret_token,
            active_websockets: Arc::new(DashMap::new()),
            pending_responses: Arc::new(DashMap::new()),
            allowed_paths: Arc::new(DashMap::new()),
            allowed_ips: Arc::new(DashMap::new()),
            allowed_asns: Arc::new(DashMap::new()),
            db_reader: Arc::new(RwLock::new(
                maxminddb::Reader::open_readfile(config.asn_db_path)
                    .expect("Failed to open ASN database"),
            )),
            maxmind_license_key: config.maxmind_license_key,
        }
    }
}

#[tokio::main]
async fn main() {
    let config = config::Config::new();
    let app_state = Arc::new(AppState::new(config));
    logging::setup_tracing();

    let updater_state = app_state.clone();
    asn_updater::spawn_asn_updater_task(updater_state);

    info!("Starting Simplified Rust Tunnel Server...");

    let app = Router::new()
        .route("/ws", get(websocket::ws_handler))
        .route("/*path", any(forwarding::forward_handler))
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
