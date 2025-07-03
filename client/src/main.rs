mod models;
mod config;
mod utils;

use crate::models::{TunneledHttpRequest, TunneledHttpResponse};

use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message as WsMessage,
    tungstenite,
};
use url::Url;
use tracing::{info, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;
use tungstenite::http::HeaderValue;
use tungstenite::http::header::{AUTHORIZATION};
use tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use config::AppConfig;

#[tokio::main]
async fn main() {

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tunnel_client=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Rust Tunnel Client...");
    
    let config = AppConfig::new();

    let url = Url::parse(&format!("{}?client_id={}", config.server_ws_url, config.client_id))
        .expect("Failed to parse WebSocket URL. Please ensure it's a valid URL.");

    let auth_header_value = format!("Bearer {}", config.secret_token);

    let host = url.host_str().expect("Invalid host in WebSocket URL. Please ensure the URL has a host.");

    let request = Request::builder()
        .method("GET")
        .uri(url.as_str())
        .header("Host", host)
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13")
        .header(AUTHORIZATION, HeaderValue::from_str(&auth_header_value).expect("Invalid Authorization header value. This should not happen with valid input."))
        .body(())
        .unwrap();


    println!("Attempting to connect to WebSocket server");

    let (mut ws_stream, response) = match connect_async(request).await {
        Ok((stream, response)) => (stream, response),
        Err(e) => {
            error!("Failed to connect to WebSocket server: {:?}", e);
            eprintln!("\nERROR: Could not connect. Please check the server URL, client ID, token, and ensure the server is running.");
            return;
        }
    };

    println!("WebSocket connection established!");

    let client_url = config.server_ws_url.replace("ws://", "https://").replace("ws", &config.client_id);

    println!("You can now send messages to the server on: {}", client_url);

    debug!("Server response during handshake: {:?}", response);

    loop {
        tokio::select! {
            message = ws_stream.next() => {
                match message {
                    Some(Ok(WsMessage::Text(text))) => {
                        info!("Received text from server: {}", text);
                        if let Err(e) = ws_stream.send(WsMessage::Text(format!("Client received: {}", text).into())).await {
                            error!("Failed to echo message back to server: {:?}", e);
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Binary(data))) => {
                        info!("Received binary message ({} bytes). (Full tunneling logic not active in this snippet)", data.len());
                        // If you plan to re-integrate the full reqwest tunneling logic, this is where you'd use `target_http_service_url`.
                        // Example:
                        /*
                        match serde_json::from_slice::<TunneledHttpRequest>(&data) {
                            Ok(tunneled_req) => {
                                info!("Received TunneledHttpRequest for path: {}", tunneled_req.path);
                                let client = reqwest::Client::new(); // Or reuse a client
                                let request_builder = client.request(
                                    reqwest::Method::from_bytes(tunneled_req.method.as_bytes()).unwrap_or(reqwest::Method::GET),
                                    format!("{}{}", target_http_service_url, tunneled_req.path),
                                );
                                // ... add headers, body, send request, build TunneledHttpResponse, send back over ws_stream
                            },
                            Err(e) => {
                                error!("Failed to deserialize TunneledHttpRequest: {}", e);
                            }
                        }
                        */
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        debug!("Received PING from server. Sending PONG.");
                        if let Err(e) = ws_stream.send(WsMessage::Pong(data)).await {
                            error!("Failed to send PONG to server: {:?}", e);
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Close(close_frame))) => {
                        info!("Received Close frame from server: {:?}", close_frame);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket read error: {:?}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket connection closed by server (stream finished).");
                        break;
                    }
                    _ => {
                        debug!("Received unsupported message type from server.");
                    }
                }
            }
        }
    }

    info!("Rust Tunnel Client shutting down.");
}