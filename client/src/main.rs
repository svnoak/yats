mod config;
mod http_handler;
mod models;
mod utils;
mod websocket_handler;

use crate::websocket_handler::handle_websocket_messages;
use config::AppConfig;
use futures_util::SinkExt;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use websocket_handler::connect_to_websocket;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tunnel_client=debug,reqwest=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Rust Tunnel Client...");

    let config = AppConfig::new();

    let (mut ws_sender, ws_receiver) = match connect_to_websocket(&config).await {
        Ok((sender, receiver)) => (sender, receiver),
        Err(e) => {
            error!("Failed to connect: {:?}", e);
            eprintln!(
                "\nERROR: Could not connect. Please check the server URL, client ID, token, and ensure the server is running."
            );
            return;
        }
    };

    let (tx, mut rx) = mpsc::channel::<WsMessage>(100);

    let tx_ctrlc = tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("Ctrl-C received, sending Close frame to server...");
            let _ = tx_ctrlc.send(WsMessage::Close(None)).await;
        }
    });

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = ws_sender.send(message).await {
                error!("Failed to send message over WebSocket: {:?}", e);
                break;
            }
        }
        info!("WebSocket sender task shutting down.");
    });

    let client_public_url_base = config
        .server_ws_url
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .trim_end_matches("/ws")
        .to_string();

    println!("\nYour tunnel is active! Requests to:");
    config.allowed_paths.iter().for_each(|path| {
        println!("  {}/{}{}", client_public_url_base, config.client_id, path);
    });
    
    println!(
        "Will be forwarded to your local service at: {}",
        config.target_http_service_url
    );

    let http_client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build reqwest client");

    handle_websocket_messages(ws_receiver, tx, http_client, config).await;

    info!("Rust Tunnel Client shutting down.");
}
