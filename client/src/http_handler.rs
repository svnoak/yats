use crate::models::{TunneledHttpResponse, TunneledRequest};
use base64::{engine::general_purpose, Engine};
use reqwest::{Client, Method as ReqwestMethod};
use tungstenite::http::HeaderValue;
use tracing::{error, info, warn};

pub async fn forward_request_to_local_service(
    http_client: &Client,
    tunneled_req: TunneledRequest,
    target_http_service_url: &str,
) -> TunneledHttpResponse {
    let local_service_url = format!("{}{}", target_http_service_url, tunneled_req.path);
    info!(
        "Forwarding request (ID: {}) to local service: {} {}",
        tunneled_req.id, tunneled_req.method, local_service_url
    );

    let method = match ReqwestMethod::from_bytes(tunneled_req.method.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            error!(
                "Invalid HTTP method received for ID {}: {}",
                tunneled_req.id, tunneled_req.method
            );
            return TunneledHttpResponse {
                id: tunneled_req.id,
                status: 400,
                headers: std::collections::HashMap::new(),
                body: Some(general_purpose::STANDARD.encode("Invalid HTTP method")),
            };
        }
    };

    let mut request_builder = http_client.request(method, &local_service_url);

    if !tunneled_req.query_params.is_empty() {
        request_builder = request_builder.query(&tunneled_req.query_params);
    }

    for (key, value) in tunneled_req.headers {
        if key.eq_ignore_ascii_case("host")
            || key.eq_ignore_ascii_case("connection")
            || key.eq_ignore_ascii_case("keep-alive")
            || key.eq_ignore_ascii_case("proxy-authenticate")
            || key.eq_ignore_ascii_case("proxy-authorization")
            || key.eq_ignore_ascii_case("te")
            || key.eq_ignore_ascii_case("trailer")
            || key.eq_ignore_ascii_case("transfer-encoding")
            || key.eq_ignore_ascii_case("upgrade")
        {
            continue;
        }
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            request_builder = request_builder.header(&key, header_value);
        } else {
            warn!(
                "Skipping invalid header value for key '{}' (ID {}): {}",
                key, tunneled_req.id, value
            );
        }
    }

    if let Some(body_str) = tunneled_req.body {
        if !body_str.is_empty() {
            match general_purpose::STANDARD.decode(&body_str) {
                Ok(decoded_body) => {
                    request_builder = request_builder.body(decoded_body);
                }
                Err(e) => {
                    error!(
                        "Failed to base64 decode request body for ID {}: {}",
                        tunneled_req.id, e
                    );
                    return TunneledHttpResponse {
                        id: tunneled_req.id,
                        status: 400,
                        headers: std::collections::HashMap::new(),
                        body: Some(general_purpose::STANDARD.encode("Failed to decode request body")),
                    };
                }
            }
        }
    }

    match request_builder.send().await {
        Ok(resp) => {
            info!(
                "Received response from local service for ID {}. Status: {}",
                tunneled_req.id,
                resp.status()
            );
            let status = resp.status().as_u16();
            let mut headers_map = std::collections::HashMap::new();
            for (key, value) in resp.headers() {
                headers_map
                    .insert(key.to_string(), value.to_str().unwrap_or_default().to_string());
            }

            let body_bytes = match resp.bytes().await {
                Ok(bytes) => bytes.to_vec(),
                Err(e) => {
                    error!(
                        "Failed to read response body from local service for ID {}: {:?}",
                        tunneled_req.id, e
                    );
                    Vec::new()
                }
            };
            let body_base64 = general_purpose::STANDARD.encode(body_bytes);

            TunneledHttpResponse {
                id: tunneled_req.id,
                status,
                headers: headers_map,
                body: Some(body_base64),
            }
        }
        Err(e) => {
            error!(
                "Failed to send request to local service for ID {}: {:?}",
                tunneled_req.id, e
            );
            TunneledHttpResponse {
                id: tunneled_req.id,
                status: 503,
                headers: std::collections::HashMap::new(),
                body: Some(general_purpose::STANDARD.encode(format!("Service Unavailable"))),
            }
        }
    }
}
