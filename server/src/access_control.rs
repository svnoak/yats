use crate::{models::ClientParams, AppState};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::{headers::Authorization, TypedHeader};
use ipnetwork::IpNetwork;
use maxminddb::geoip2;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{error, warn};

pub fn authenticate_client(
    auth_header: Option<TypedHeader<Authorization<axum_extra::headers::authorization::Bearer>>>,
    params: &ClientParams,
    app_state: &Arc<AppState>,
) -> Result<(), impl IntoResponse> {
    let auth_header = if let Some(TypedHeader(auth_header)) = auth_header {
        auth_header
    } else {
        error!("Missing Authorization header");
        return Err((StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response());
    };

    if auth_header.token() != app_state.secret_token {
        error!("Invalid token provided: {}", auth_header.token());
        return Err((StatusCode::FORBIDDEN, "Invalid token").into_response());
    }

    if app_state.active_websockets.contains_key(&params.client_id) {
        error!(
            "Client ID '{}' already exists. Rejecting connection.",
            params.client_id
        );
        return Err((StatusCode::CONFLICT, "Client ID already connected").into_response());
    }

    Ok(())
}

pub fn add_allowed_ips(
    app_state: &Arc<AppState>,
    client_id: &str,
    ips: Vec<String>,
) -> Result<(), Response> {
    app_state.allowed_ips.insert(client_id.to_string(), ips);
    Ok(())
}

pub fn add_allowed_paths(
    app_state: &Arc<AppState>,
    client_id: &str,
    paths: Vec<String>,
) -> Result<(), impl IntoResponse> {
    if paths.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No paths provided").into_response());
    }

    app_state.allowed_paths.insert(client_id.to_string(), paths);
    Ok(())
}

pub fn add_allowed_asns(
    app_state: &Arc<AppState>,
    client_id: &str,
    asns: Vec<u32>,
) -> Result<(), Response> {
    app_state.allowed_asns.insert(client_id.to_string(), asns);
    Ok(())
}

pub fn is_ip_allowed(
    app_state: &Arc<AppState>,
    client_id: &str,
    remote_ip: IpAddr,
) -> Result<(), Response> {
    if let Some(allowed_ips_ref) = app_state.allowed_ips.get(client_id) {
        if allowed_ips_ref.is_empty() {
            return Ok(());
        }

        let is_allowed = allowed_ips_ref.iter().any(|ip_str| {
            if let Ok(network) = ip_str.parse::<IpNetwork>() {
                network.contains(remote_ip)
            } else {
                false
            }
        });

        if is_allowed {
            Ok(())
        } else {
            Err((StatusCode::FORBIDDEN, "IP not allowed").into_response())
        }
    } else {
        Ok(())
    }
}

pub fn is_path_allowed(
    app_state: &Arc<AppState>,
    client_id: &str,
    requested_path: &str,
) -> Result<(), impl IntoResponse> {
    if let Some(allowed_paths_ref) = app_state.allowed_paths.get(client_id) {
        if allowed_paths_ref.is_empty() {
            error!("No allowed paths configured for client_id '{}'.", client_id);
            return Err(StatusCode::NOT_FOUND.into_response());
        }

        let is_allowed = allowed_paths_ref.iter().any(|p| p == requested_path);

        if is_allowed {
            Ok(())
        } else {
            error!(
                "Path '{}' is not in the allowed list for client_id '{}'",
                requested_path, client_id
            );
            Err(StatusCode::NOT_FOUND.into_response())
        }
    } else {
        error!(
            "No path configuration found for client_id '{}'. It may be disconnected.",
            client_id
        );
        Err(StatusCode::NOT_FOUND.into_response())
    }
}

pub async fn is_asn_allowed(
    app_state: &Arc<AppState>,
    client_id: &str,
    remote_ip: IpAddr,
) -> Result<(), impl IntoResponse> {
    if let Some(allowed_asns_ref) = app_state.allowed_asns.get(client_id) {
        if allowed_asns_ref.is_empty() {
            return Ok(());
        }

        // Allow requests from loopback addresses without further checks.
        if remote_ip.is_loopback() {
            return Ok(());
        }

        let reader_guard = app_state.db_reader.read().await;
        let asn = reader_guard.lookup::<geoip2::Asn>(remote_ip);

        let asn = asn
            .inspect_err(|_e| error!("Error while doing ASN lookup. IP: {remote_ip}"))
            .map_err(|_e| (StatusCode::NOT_FOUND, "No ASN found for the given IP").into_response())?
            .ok_or_else(|| {
                warn!("No ASN connected to IP: {remote_ip}");
                (StatusCode::NOT_FOUND, "No ASN found for the given IP").into_response()
            })?
            .autonomous_system_number
            .ok_or_else(|| {
                warn!("ASN number is None for IP: {remote_ip}");
                (StatusCode::NOT_FOUND, "No ASN found for the given IP").into_response()
            })?;

        let is_allowed = allowed_asns_ref.iter().any(|p| p == &asn);

        if is_allowed {
            Ok(())
        } else {
            error!("Asn '{asn}' is not in the allowed list for client_id '{client_id}'");
            Err((StatusCode::FORBIDDEN, "ASN not allowed").into_response())
        }
    } else {
        Ok(())
    }
}
