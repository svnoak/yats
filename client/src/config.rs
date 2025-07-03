use std::env;
use dotenvy::dotenv;
use crate::utils::{generate_random_id_phrase, get_input_with_default};

pub struct AppConfig {
    pub server_ws_url: String,
    pub client_id: String,
    pub secret_token: String,
    pub target_http_service_url: String,
}

impl AppConfig {
    pub fn new() -> Self {

        dotenv().ok();

        let server_ws_url_default =
            env::var("SERVER_WS_URL").unwrap_or_else(|_| "ws://localhost:3000/ws".to_string());
        let server_ws_url =
            get_input_with_default("Enter Tunnel Server URL", &server_ws_url_default);

        let client_id_default = generate_random_id_phrase();
        let client_id = get_input_with_default("Choose Client ID", &client_id_default);

        let secret_token_default =
            env::var("SECRET_TOKEN").unwrap_or_else(|_| "your_secret_token".to_string());
        let secret_token = get_input_with_default("Enter Secret Token", &secret_token_default);

        let target_http_service_url_default = env::var("TARGET_HTTP_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let target_http_service_url = get_input_with_default(
            "Enter Target HTTP Service URL",
            &target_http_service_url_default,
        );

        Self {
            server_ws_url,
            client_id,
            secret_token,
            target_http_service_url,
        }
    }
}