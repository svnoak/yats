use crate::utils::{generate_random_id_phrase, get_input_with_default};
use dotenvy::dotenv;
use ipnetwork::IpNetwork;
use std::{env, io};
use url::Url;

pub struct AppConfig {
    pub server_ws_url: String,
    pub client_id: String,
    pub secret_token: String,
    pub target_http_service_url: String,
    pub allowed_paths: Vec<String>,
    pub allowed_ips: Vec<String>,
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

        let allowed_ips = get_allowed_ips();

        Self {
            server_ws_url,
            client_id,
            secret_token,
            target_http_service_url,
            allowed_paths,
            allowed_ips,
        }
    }
}

fn get_allowed_ips() -> Vec<String> {
    println!(
        "\n▶ Enter allowed IPs or CIDR ranges for the tunnel (e.g., 192.168.1.1, 10.0.0.0/8)."
    );
    println!("  - Press Enter on an empty line to finish. If no IPs are provided, all IPs will be allowed.");

    let mut ips = Vec::new();
    loop {
        print!("> ");
        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");

        let mut ip_input = String::new();
        match io::stdin().read_line(&mut ip_input) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let ip_input = ip_input.trim().to_string();
                if ip_input.is_empty() {
                    if ips.is_empty() {
                        print!("  ⚠️ Are you sure you want to allow all IPs? This is a security risk. (y/N) ");
                        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
                        let mut confirmation = String::new();
                        io::stdin()
                            .read_line(&mut confirmation)
                            .expect("Failed to read line");
                        if confirmation.trim().eq_ignore_ascii_case("y") {
                            println!("  ✅ All IPs will be allowed.");
                            return Vec::new();
                        } else {
                            println!("  Operation cancelled. Please enter at least one IP or CIDR range.");
                            continue;
                        }
                    } else {
                        break;
                    }
                }

                match ip_input.parse::<IpNetwork>() {
                    Ok(_) => {
                        if !ips.contains(&ip_input) {
                            ips.push(ip_input);
                            println!("  ✅ Added.");
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "  ❌ Error: Invalid IP or CIDR range: {}. Please try again.",
                            e
                        );
                    }
                }
            }
            Err(_) => {
                eprintln!("Error: Failed to read input.");
                break;
            }
        }
    }
    ips
}

fn get_allowed_paths() -> Vec<String> {
    println!("\n▶ Enter the URL paths to allow access to from the public URL.");
    println!("  - Standard paths should start with a '/' (e.g., /api/v1).");
    println!("  - To allow the root URL with no trailing slash (e.g., /client-id), type <root>.");
    println!("  - Press Enter on an empty line to finish.");

    let mut paths = Vec::new();
    loop {
        print!("> ");
        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");

        let mut path = String::new();
        match io::stdin().read_line(&mut path) {
            Ok(0) => break, // EOF, break loop
            Ok(_) => {
                let path = path.trim().to_string();

                if path.is_empty() {
                    if paths.is_empty() {
                        println!("  ❌ No paths provided. '/' will be used as the default path.");
                        paths.push("/".to_string());
                        println!("  ✅ Added default path: '/'.");
                    }
                    break;
                }

                if path.to_lowercase() == "<root>" {
                    if !paths.contains(&"".to_string()) {
                        paths.push("".to_string());
                        println!("  ✅ Added root path (no trailing slash).");
                    }
                    continue;
                }

                if path.starts_with('/') {
                    if !paths.contains(&path) {
                        println!("  ✅ Added path: '{}'", path);
                        paths.push(path);
                    }
                } else {
                    loop {
                        print!(
                            "  🤔 Warning: The path '{}' is non-standard because it doesn't start with a '/'.\n  Are you sure you want to add it? (y/N): ",
                            path
                        );
                        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");

                        let mut confirmation = String::new();
                        io::stdin()
                            .read_line(&mut confirmation)
                            .expect("Failed to read confirmation");

                        match confirmation.trim().to_lowercase().as_str() {
                            "y" | "yes" => {
                                if !paths.contains(&path) {
                                    paths.push(path.clone());
                                }
                                println!("  ✅ Added non-standard path.");
                                break;
                            }
                            "n" | "" => {
                                println!("  ❌ Path was not added.");
                                break;
                            }
                            _ => println!("  Invalid input. Please enter 'y' or 'n'."),
                        }
                    }
                }
            }
            Err(_) => {
                eprintln!("Error: Failed to read input. Please ensure it's valid UTF-8.");
                break;
            }
        }
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
                    eprintln!("Error: The URL scheme must be 'http' or 'https'. Please try again.");
                } else {
                    return url_input;
                }
            }
            Err(e) => {
                eprintln!("Error: Invalid URL ({}). Please enter a full URL (e.g., http://localhost:8080).", e);
            }
        }
    }
}
