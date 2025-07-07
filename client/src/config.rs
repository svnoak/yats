use crate::utils::{generate_random_id_phrase, get_input_with_default};
use dotenvy::dotenv;
use std::{env, io};
use url::Url;

pub struct AppConfig {
    pub server_ws_url: String,
    pub client_id: String,
    pub secret_token: String,
    pub target_http_service_url: String,
    pub allowed_paths: Vec<String>,
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

        let target_http_service_url = get_target_local_url();

        let allowed_paths = get_allowed_paths();

        Self {
            server_ws_url,
            client_id,
            secret_token,
            target_http_service_url,
            allowed_paths,
        }
    }
}

fn get_allowed_paths() -> Vec<String> {
    println!("\nEnter the URL paths you want to allow access to from the public URL.");
    println!("Paths should start with a '/' (e.g., /api/v1/users).");
    println!("Press Enter on an empty line to finish.");

    let mut paths = Vec::new();
    loop {
        let mut path = String::new();
        print!("> ");
        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
        io::stdin()
            .read_line(&mut path)
            .expect("Failed to read line");
        let path = path.trim().to_string();

        if path.is_empty() {
            break;
        }

        if !path.starts_with('/') {
            println!("Warning: Path '{}' does not start with a '/'. It will be added, but this is not standard.", path);
        }

        paths.push(path);
    }

    if paths.is_empty() {
        println!("No paths entered. Defaulting to the root path '/'.");
        paths.push("/".to_string());
    }

    paths
}

fn get_target_local_url() -> String {
    let target_http_service_url_default =
        env::var("TARGET_HTTP_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    loop {
        let url_input = get_input_with_default(
            "Enter Target HTTP Service URL",
            &target_http_service_url_default,
        );

        match Url::parse(&url_input) {
            Ok(parsed_url) => {
                if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
                    eprintln!("Warning: The URL must start with 'http://' or 'https://'. Please try again.");
                } else {
                    return url_input;
                }
            }
            Err(e) => {
                eprintln!("Invalid URL: {}. Please try again.", e);
            }
        }
    }
}
