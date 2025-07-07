use crate::{models::ClientParams, AppState};
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::{headers::Authorization, TypedHeader};
use std::sync::Arc;
use tracing::error;

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
