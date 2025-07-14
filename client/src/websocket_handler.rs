use crate::config::AppConfig;
use crate::http_handler::forward_request_to_local_service;
use crate::models::{TunneledHttpResponse, TunneledRequest};
use futures_util::stream::{SplitSink, SplitStream, StreamExt};
use reqwest::Client;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info};
use tungstenite::handshake::client::Request;
use tungstenite::http::header::AUTHORIZATION;
use tungstenite::http::HeaderValue;
use url::Url;

pub type WsSender = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;
pub type WsReceiver = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn connect_to_websocket(
    config: &AppConfig,
) -> Result<(WsSender, WsReceiver), Box<dyn std::error::Error>> {
    let mut ws_url = Url::parse(&config.server_ws_url)?;
    ws_url
        .query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("allowed_paths", &config.allowed_paths.join(","));

    if !config.allowed_ips.is_empty() {
        ws_url
            .query_pairs_mut()
            .append_pair("allowed_ips", &config.allowed_ips.join(","));
    }

    let auth_header_value = format!("Bearer {}", config.secret_token);
    let host = ws_url.host_str().ok_or("Invalid WebSocket URL: no host")?;

    let request = Request::builder()
        .method("GET")
        .uri(ws_url.as_str())
        .header("Host", host)
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13")
        .header(AUTHORIZATION, HeaderValue::from_str(&auth_header_value)?)
        .body(())?;

    info!("Connecting to WebSocket server at {}", ws_url);

    let (ws_stream, response) = connect_async(request).await?;

    debug!("Server response during handshake: {:?}", response);
    info!("WebSocket connection established!");

    Ok(ws_stream.split())
}

pub async fn handle_websocket_messages(
    mut ws_receiver: WsReceiver,
    tx: mpsc::Sender<WsMessage>,
    http_client: Client,
    config: AppConfig,
) {
    loop {
        tokio::select! {
            message = ws_receiver.next() => {
                match message {
                    Some(Ok(WsMessage::Text(text))) => {
                        info!("Received text from server: {}", text);
                        let tx_clone = tx.clone();
                        let http_client_clone = http_client.clone();
                        let config_clone = config.clone();

                        tokio::spawn(async move {
                            match serde_json::from_str::<TunneledRequest>(&text) {
                                Ok(tunneled_req) => {
                                    let response = forward_request_to_local_service(&http_client_clone, tunneled_req, &config_clone.target_http_service_url).await;
                                    match serde_json::to_string(&response) {
                                        Ok(json_payload) => {
                                            if let Err(e) = tx_clone.send(WsMessage::Text(json_payload)).await {
                                                error!("Failed to send response back to server (ID: {}): {:?}", response.id, e);
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to serialize response (ID: {}): {:?}", response.id, e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to deserialize request from server: {}", e);
                                    let err_resp = TunneledHttpResponse {
                                        id: "unknown".to_string(),
                                        status: 400,
                                        headers: std::collections::HashMap::new(),
                                        body: Some(format!("Failed to deserialize request: {}", e)),
                                    };
                                     if let Ok(json_payload) = serde_json::to_string(&err_resp) {
                                        if let Err(e) = tx_clone.send(WsMessage::Text(json_payload)).await {
                                            error!("Failed to send deserialization error back to server: {:?}", e);
                                        }
                                    }
                                }
                            }
                        });
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
                        info!("WebSocket connection closed by server.");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Clone for AppConfig {
    fn clone(&self) -> Self {
        Self {
            server_ws_url: self.server_ws_url.clone(),
            client_id: self.client_id.clone(),
            secret_token: self.secret_token.clone(),
            target_http_service_url: self.target_http_service_url.clone(),
            allowed_paths: self.allowed_paths.clone(),
            allowed_ips: self.allowed_ips.clone(),
        }
    }
}
