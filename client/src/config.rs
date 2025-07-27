use crate::config_manager::save_configs;
use crate::utils::{generate_random_id_phrase, get_input_with_default};
use dotenvy::dotenv;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    env,
    io::{self, Write},
};
use tracing::{error, info};
use url::Url;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub server_ws_url: String,
    pub client_id: String,
    pub secret_token: String,
    pub target_http_service_url: String,
    pub allowed_paths: Vec<String>,
    pub allowed_ips: Vec<String>,
    pub allowed_asns: Vec<u32>,
}

/// The main entry point for configuration.
/// It determines whether to show a creation wizard or the selection menu.
pub async fn get_or_create_config(
    stored_configs: &mut HashMap<String, AppConfig>,
) -> Option<AppConfig> {
    if stored_configs.is_empty() {
        // Flow for first-time run or no saved configs
        println!("\n--- Tunnel Client Setup ---");
        println!("No saved configurations found. Let's create a new one.");
        let new_config = gather_new_config();

        print!("\nDo you want to save this configuration for future use? (y/N): ");
        io::stdout().flush().unwrap();
        let mut save_choice = String::new();
        io::stdin().read_line(&mut save_choice).unwrap();

        if save_choice.trim().eq_ignore_ascii_case("y") {
            prompt_and_save_config_name(stored_configs, &new_config);
        }
        Some(new_config)
    } else {
        // Flow for when saved configs exist
        select_or_create_config(stored_configs).await
    }
}

/// Manages the user menu for selecting, creating, or deleting configurations.
async fn select_or_create_config(
    stored_configs: &mut HashMap<String, AppConfig>,
) -> Option<AppConfig> {
    loop {
        println!("\n--- Tunnel Client Configuration ---");
        println!("Available configurations:");
        let mut sorted_keys: Vec<_> = stored_configs.keys().collect();
        sorted_keys.sort();

        if stored_configs.is_empty() {
            println!("\n- None. Please create a new configuration.");
        } else {
            for (i, name) in sorted_keys.iter().enumerate() {
                println!("  {}. {}", i + 1, name);
            }
        }

        println!("\nOptions:");
        println!("  c - Create a new configuration");
        println!("  d - Delete a configuration");
        println!("  q - Quit");
        print!("\nEnter a number to use a config, or choose an option: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();

        match choice {
            "c" => {
                let new_config = gather_new_config();
                print!("\nDo you want to save this new configuration? (y/N): ");
                io::stdout().flush().unwrap();
                let mut save_choice = String::new();
                io::stdin().read_line(&mut save_choice).unwrap();

                if save_choice.trim().eq_ignore_ascii_case("y") {
                    prompt_and_save_config_name(stored_configs, &new_config);
                }
                return Some(new_config);
            }
            "d" => {
                print!("Enter the number of the configuration to delete: ");
                io::stdout().flush().unwrap();
                let mut del_choice = String::new();
                io::stdin().read_line(&mut del_choice).unwrap();
                if let Ok(num) = del_choice.trim().parse::<usize>() {
                    if num > 0 && num <= sorted_keys.len() {
                        let key_to_remove = sorted_keys[num - 1].clone();
                        stored_configs.remove(&key_to_remove);
                        if let Err(e) = save_configs(stored_configs) {
                            error!("Failed to save changes: {}", e);
                        } else {
                            println!("Configuration '{}' deleted.", key_to_remove);
                        }
                    } else {
                        println!("Invalid number.");
                    }
                } else {
                    println!("Invalid input.");
                }
            }
            "q" => return None,
            _ => {
                if let Ok(num) = choice.parse::<usize>() {
                    if num > 0 && num <= sorted_keys.len() {
                        let key = sorted_keys[num - 1];
                        return Some(stored_configs.get(key).unwrap().clone());
                    }
                }
                println!("Invalid selection. Please try again.");
            }
        }
    }
}

/// Gathers all configuration details interactively from the user.
fn gather_new_config() -> AppConfig {
    dotenv().ok();

    let server_ws_url_default =
        env::var("SERVER_WS_URL").unwrap_or_else(|_| "ws://localhost:3000/ws".to_string());
    let server_ws_url = get_input_with_default("Enter Tunnel Server URL", &server_ws_url_default);

    let client_id_default = generate_random_id_phrase();
    let client_id = get_input_with_default("Choose Client ID", &client_id_default);

    let secret_token_default =
        env::var("SECRET_TOKEN").unwrap_or_else(|_| "your_secret_token".to_string());
    let secret_token = get_input_with_default("Enter Secret Token", &secret_token_default);

    let target_http_service_url = get_target_local_url();
    let allowed_paths = get_allowed_paths();
    let allowed_ips = get_allowed_ips();
    let allowed_asns = get_allowed_asns();

    AppConfig {
        server_ws_url,
        client_id,
        secret_token,
        target_http_service_url,
        allowed_paths,
        allowed_ips,
        allowed_asns,
    }
}

/// Prompts the user for a configuration name and saves the configuration if valid.
fn prompt_and_save_config_name(
    stored_configs: &mut HashMap<String, AppConfig>,
    new_config: &AppConfig,
) {
    loop {
        print!("Enter a name for this configuration: ");
        io::stdout().flush().unwrap();
        let mut config_name = String::new();
        io::stdin().read_line(&mut config_name).unwrap();
        let config_name = config_name.trim().to_string();

        if config_name.is_empty() {
            println!("Configuration name cannot be empty. Please try again.");
            continue;
        }
        if stored_configs.contains_key(&config_name) {
            println!(
                "Configuration name '{}' already exists. Please choose a different name.",
                config_name
            );
            continue;
        }

        stored_configs.insert(config_name.clone(), new_config.clone());
        if let Err(e) = save_configs(stored_configs) {
            error!("Failed to save configuration: {}", e);
        } else {
            info!("Configuration '{}' saved.", config_name);
        }

        break;
    }
}

// --- Helper functions for gathering input ---

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

fn get_allowed_asns() -> Vec<u32> {
    println!("\n▶ Enter allowed ASNs for the tunnel (e.g., AS15169).");
    println!("  - Press Enter on an empty line to finish. If no ASNs are provided, all ASNs will be allowed.");

    let mut asns = Vec::new();
    loop {
        print!("> ");
        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");

        let mut asn_input = String::new();
        match io::stdin().read_line(&mut asn_input) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let asn_input = asn_input.trim().to_string();
                if asn_input.is_empty() {
                    if asns.is_empty() {
                        print!("  ⚠️ Are you sure you want to allow all ASNs? This is a security risk. (y/N) ");
                        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
                        let mut confirmation = String::new();
                        io::stdin()
                            .read_line(&mut confirmation)
                            .expect("Failed to read line");
                        if confirmation.trim().eq_ignore_ascii_case("y") {
                            println!("  ✅ All ASNs will be allowed.");
                            return Vec::new();
                        } else {
                            println!("  Operation cancelled. Please enter at least one ASN.");
                            continue;
                        }
                    } else {
                        break;
                    }
                }

                match validate_asn(&asn_input) {
                    Ok(ref asn) => {
                        if !asns.contains(asn) {
                            asns.push(*asn);
                            println!("  ✅ Added.");
                        }
                    }
                    Err(e) => eprintln!("  ❌ Error: {e}. Please try again."),
                }
            }
            Err(_) => {
                eprintln!("Error: Failed to read input.");
                break;
            }
        }
    }
    asns
}

fn validate_asn(asn_str: &str) -> Result<u32, String> {
    let cleaned = asn_str.trim().to_uppercase();

    let number_str = match cleaned.strip_prefix("AS") {
        Some(s) => s,
        None => &cleaned,
    };

    match number_str.parse::<u32>() {
        // The size of an ASN can be up to 32 bits
        Ok(asn) if asn >= 1 => Ok(asn),
        Ok(_) => Err("ASN out of valid range (1-4294967295)".to_string()),
        Err(_) => Err("Invalid ASN format".to_string()),
    }
}
