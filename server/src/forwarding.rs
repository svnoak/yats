use crate::models::TunneledRequest;
use crate::{access_control, AppState};
use axum::extract::ws::Message;
use axum::extract::{ConnectInfo, State};
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose;
use base64::Engine;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{error, info};
use uuid::Uuid;

async fn handle_forwarding_request(
    app_state: Arc<AppState>,
    client_id: String,
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
    forward_path: String,
    query_params: HashMap<String, String>,
    remote_ip: IpAddr,
) -> Response {
    info!(
        "Forwarding request for client_id: {}, path: {}, method: {}, query_params: {:?}",
        client_id,
        forward_path,
        method.as_str(),
        query_params
    );

    if let Err(response) = access_control::is_ip_allowed(&app_state, &client_id, remote_ip) {
        return response;
    }

    if let Err(response) = access_control::is_path_allowed(&app_state, &client_id, &forward_path) {
        return response.into_response();
    }

    if let Some(ws_sender) = app_state.active_websockets.get(&client_id) {
        let headers_map: HashMap<String, String> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();

        let request_id = Uuid::new_v4().to_string();
        let tunneled_request = TunneledRequest {
            id: request_id.clone(),
            method: method.to_string(),
            path: forward_path,
            headers: headers_map,
            query_params,
            body: general_purpose::STANDARD.encode(body),
        };

        let (tx, rx) = oneshot::channel();
        app_state.pending_responses.insert(request_id.clone(), tx);

        match serde_json::to_string(&tunneled_request) {
            Ok(json_payload) => {
                if let Err(e) = ws_sender.send(Message::Text(json_payload)).await {
                    error!("Failed to send request to websocket: {}", e);
                    app_state.pending_responses.remove(&request_id);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to forward request to client",
                    )
                        .into_response();
                }

                match tokio::time::timeout(tokio::time::Duration::from_secs(30), rx).await {
                    Ok(Ok(response)) => {
                        let mut builder = axum::response::Response::builder().status(
                            StatusCode::from_u16(response.status)
                                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                        );

                        for (key, value) in response.headers {
                            builder = builder.header(key, value);
                        }

                        let body = response
                            .body
                            .and_then(|b| general_purpose::STANDARD.decode(b).ok());
                        builder
                            .body(axum::body::Body::from(body.unwrap_or_default()))
                            .unwrap_or_else(|_| {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to build response",
                                )
                                    .into_response()
                            })
                    }
                    Ok(Err(_)) | Err(_) => {
                        app_state.pending_responses.remove(&request_id);
                        (StatusCode::GATEWAY_TIMEOUT, "Request to client timed out").into_response()
                    }
                }
            }
            Err(e) => {
                error!("Failed to serialize request: {}", e);
                app_state.pending_responses.remove(&request_id);
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
pub async fn forward_handler(
    State(app_state): State<Arc<AppState>>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Path(path): Path<String>,
    Query(query_params): Query<HashMap<String, String>>,
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let mut segments = path.splitn(2, '/');
    let client_id = segments.next().unwrap_or_default().to_string();

    if client_id.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing client_id in path").into_response();
    }

    let forward_path = match segments.next() {
        Some(p) => format!("/{}", p),
        None => "".to_string(),
    };

    let remote_ip = headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or(remote_addr.ip());

    handle_forwarding_request(
        app_state,
        client_id,
        method,
        headers,
        body,
        forward_path,
        query_params,
        remote_ip,
    )
    .await
}
