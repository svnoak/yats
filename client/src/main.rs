// src/main.rs

mod models;
mod config;
mod utils;

use crate::models::{TunneledRequest, TunneledHttpResponse};

use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message as WsMessage,
    tungstenite,
};
use url::Url;
use tracing::{info, error, debug, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use futures_util::stream::{StreamExt};
use futures_util::sink::SinkExt;
use tungstenite::http::HeaderValue;
use tungstenite::http::header::{AUTHORIZATION};
use tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use config::AppConfig;
use reqwest::{Client, Method as ReqwestMethod};
use base64::engine::general_purpose;
use base64::Engine;
use std::time::Duration;
use tokio::sync::mpsc;

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

    let ws_url = Url::parse(&format!("{}?client_id={}", config.server_ws_url, config.client_id))
        .expect("Failed to parse WebSocket URL. Please ensure it's a valid URL.");

    let auth_header_value = format!("Bearer {}", config.secret_token);

    let host = ws_url.host_str().expect("Invalid host in WebSocket URL. Please ensure the URL has a host.");

    let request = Request::builder()
        .method("GET")
        .uri(ws_url.as_str())
        .header("Host", host)
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13")
        .header(AUTHORIZATION, HeaderValue::from_str(&auth_header_value).expect("Invalid Authorization header value. This should not happen with valid input."))
        .body(())
        .unwrap();


    println!("Attempting to connect to WebSocket server at {}", ws_url);

    let (ws_stream, response) = match connect_async(request).await {
        Ok((stream, response)) => (stream, response),
        Err(e) => {
            error!("Failed to connect to WebSocket server: {:?}", e);
            eprintln!("\nERROR: Could not connect. Please check the server URL, client ID, token, and ensure the server is running.");
            return;
        }
    };

    println!("WebSocket connection established!");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (tx, mut rx) = mpsc::channel::<WsMessage>(100);

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = ws_sender.send(message).await {
                error!("Failed to send message over WebSocket: {:?}", e);
                break;
            }
        }
        info!("WebSocket sender task shutting down.");
    });

    let client_public_url_base = config.server_ws_url
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .trim_end_matches("/ws")
        .to_string();

    println!("\nYour tunnel is active! Requests to:");
    println!("  {}/{}", client_public_url_base, config.client_id);
    println!("  {}/{}/any/path/you/want", client_public_url_base, config.client_id);
    println!("Will be forwarded to your local service at: {}", config.target_http_service_url);

    debug!("Server response during handshake: {:?}", response);

    let http_client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build reqwest client");

    loop {
        tokio::select! {
            message = ws_receiver.next() => {
                match message {
                    Some(Ok(WsMessage::Text(text))) => {
                        info!("Received text from server (potential TunneledRequest): {}", text);
                        let tx_clone = tx.clone();
                        match serde_json::from_str::<TunneledRequest>(&text) {
                            Ok(tunneled_req) => {
                                debug!("Deserialized TunneledRequest (ID: {}): {:?}", tunneled_req.id, tunneled_req);

                                let local_service_url = format!("{}{}", config.target_http_service_url, tunneled_req.path);
                                info!("Forwarding request (ID: {}) to local service: {} {}", tunneled_req.id, tunneled_req.method, local_service_url);

                                let method = match ReqwestMethod::from_bytes(tunneled_req.method.as_bytes()) {
                                    Ok(m) => m,
                                    Err(_) => {
                                        error!("Invalid HTTP method received for ID {}: {}", tunneled_req.id, tunneled_req.method);
                                        let err_resp = TunneledHttpResponse {
                                            id: tunneled_req.id.clone(), // Clone the ID
                                            status: 400,
                                            headers: std::collections::HashMap::new(),
                                            body: Some(general_purpose::STANDARD.encode("Invalid HTTP method")),
                                        };
                                        if let Err(e) = tx_clone.send(WsMessage::Text(serde_json::to_string(&err_resp).unwrap_or_default())).await {
                                            error!("Failed to send error response back to server (ID: {}): {:?}", tunneled_req.id, e); // `tunneled_req.id` is still available here
                                        }
                                        continue;
                                    }
                                };

                                let mut request_builder = http_client.request(method, &local_service_url);

                                if !tunneled_req.query_params.is_empty() {
                                    request_builder = request_builder.query(&tunneled_req.query_params);
                                }

                                for (key, value) in tunneled_req.headers {
                                    if key.eq_ignore_ascii_case("host") ||
                                       key.eq_ignore_ascii_case("connection") ||
                                       key.eq_ignore_ascii_case("keep-alive") ||
                                       key.eq_ignore_ascii_case("proxy-authenticate") ||
                                       key.eq_ignore_ascii_case("proxy-authorization") ||
                                       key.eq_ignore_ascii_case("te") ||
                                       key.eq_ignore_ascii_case("trailer") ||
                                       key.eq_ignore_ascii_case("transfer-encoding") ||
                                       key.eq_ignore_ascii_case("upgrade") {
                                        continue;
                                    }
                                    if let Ok(header_value) = HeaderValue::from_str(&value) {
                                        request_builder = request_builder.header(&key, header_value);
                                    } else {
                                        warn!("Skipping invalid header value for key '{}' (ID {}): {}", key, tunneled_req.id, value);
                                    }
                                }

                                // Add body if present and decode from base64
                                if let Some(body_str) = tunneled_req.body {
                                    if !body_str.is_empty() {
                                        match general_purpose::STANDARD.decode(&body_str) {
                                            Ok(decoded_body) => {
                                                request_builder = request_builder.body(decoded_body);
                                            },
                                            Err(e) => {
                                                error!("Failed to base64 decode request body for ID {}: {}", tunneled_req.id, e);
                                                // Use .clone() here
                                                let err_resp = TunneledHttpResponse {
                                                    id: tunneled_req.id.clone(), // Clone the ID
                                                    status: 400,
                                                    headers: std::collections::HashMap::new(),
                                                    body: Some(general_purpose::STANDARD.encode("Failed to decode request body")),
                                                };
                                                if let Err(e) = tx_clone.send(WsMessage::Text(serde_json::to_string(&err_resp).unwrap_or_default())).await {
                                                    error!("Failed to send error response back to server (ID: {}): {:?}", tunneled_req.id, e); // `tunneled_req.id` is still available here
                                                }
                                                continue;
                                            }
                                        }
                                    }
                                }

                                let tunneled_req_id_for_spawn = tunneled_req.id.clone();

                                tokio::spawn(async move {
                                    let tunneled_http_response = match request_builder.send().await {
                                        Ok(resp) => {
                                            info!("Received response from local service for ID {}. Status: {}", tunneled_req_id_for_spawn, resp.status());
                                            let status = resp.status().as_u16();
                                            let mut headers_map = std::collections::HashMap::new();
                                            for (key, value) in resp.headers() {
                                                headers_map.insert(key.to_string(), value.to_str().unwrap_or_default().to_string());
                                            }

                                            let body_bytes = match resp.bytes().await {
                                                Ok(bytes) => bytes.to_vec(),
                                                Err(e) => {
                                                    error!("Failed to read response body from local service for ID {}: {:?}", tunneled_req_id_for_spawn, e);
                                                    Vec::new()
                                                }
                                            };
                                            let body_base64 = general_purpose::STANDARD.encode(body_bytes);

                                            TunneledHttpResponse {
                                                id: tunneled_req_id_for_spawn,
                                                status,
                                                headers: headers_map,
                                                body: Some(body_base64),
                                            }
                                        },
                                        Err(e) => {
                                            error!("Failed to send request to local service for ID {}: {:?}", tunneled_req_id_for_spawn, e);
                                            TunneledHttpResponse {
                                                id: tunneled_req_id_for_spawn,
                                                status: 503,
                                                headers: std::collections::HashMap::new(),
                                                body: Some(general_purpose::STANDARD.encode(format!("Service Unavailable"))),
                                            }
                                        }
                                    };

                                    match serde_json::to_string(&tunneled_http_response) {
                                        Ok(json_payload) => {
                                            if let Err(e) = tx_clone.send(WsMessage::Text(json_payload)).await {
                                                error!("Failed to send TunneledHttpResponse back to server (ID: {}): {:?}", tunneled_http_response.id, e);
                                            } else {
                                                info!("Successfully sent TunneledHttpResponse back to server (ID: {})", tunneled_http_response.id);
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to serialize TunneledHttpResponse (ID: {}): {:?}", tunneled_http_response.id, e);
                                        }
                                    }
                                });
                            },
                            Err(e) => {
                                error!("Failed to deserialize TunneledRequest from server: {}", e);
                                if let Err(e) = tx.send(WsMessage::Text(format!("Error: Failed to parse request: {}", e).into())).await {
                                    error!("Failed to send deserialization error back to server: {:?}", e);
                                }
                            }
                        }
                    }
                    Some(Ok(WsMessage::Binary(data))) => {
                        info!("Received binary message ({} bytes). (Not handling as TunneledRequest)", data.len());
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        debug!("Received PING from server. Sending PONG.");
                        if let Err(e) = tx.send(WsMessage::Pong(data)).await {
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