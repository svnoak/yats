use crate::config::AppConfig;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

const CONFIG_FILE_NAME: &str = "tunnel_client_configs.json";

/// Returns the platform-specific path to the configuration file.
fn get_config_path() -> Result<PathBuf, std::io::Error> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find a config directory",
        )
    })?;
    let app_config_dir = config_dir.join("tunnel_client");
    fs::create_dir_all(&app_config_dir)?;
    Ok(app_config_dir.join(CONFIG_FILE_NAME))
}

/// Loads named configurations from the JSON config file.
pub fn load_configs() -> Result<HashMap<String, AppConfig>, Box<dyn std::error::Error>> {
    let path = get_config_path()?;
    if !path.exists() {
        return Ok(HashMap::new()); // No config file yet, return empty map
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let configs = serde_json::from_reader(reader)?;
    Ok(configs)
}

/// Saves the given configurations to the JSON config file.
pub fn save_configs(
    configs: &HashMap<String, AppConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_config_path()?;
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, configs)?;
    Ok(())
}
