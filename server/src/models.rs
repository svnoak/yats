use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ClientParams {
    pub client_id: String,
    #[serde(deserialize_with = "deserialize_comma_separated")]
    pub allowed_paths: Vec<String>,
    #[serde(
        deserialize_with = "deserialize_comma_separated_optional",
        default = "default_vec"
    )]
    pub allowed_ips: Vec<String>,
    #[serde(deserialize_with = "deserialize_u32_vec", default = "default_u32_vec")]
    pub allowed_asns: Vec<u32>,
}

fn default_vec() -> Vec<String> {
    Vec::new()
}

fn default_u32_vec() -> Vec<u32> {
    Vec::new()
}

fn deserialize_u32_vec<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    s.map(|s| {
        s.split(',')
            .map(|s| s.trim().parse::<u32>())
            .collect::<Result<Vec<u32>, _>>()
    })
    .transpose()
    .map(Option::unwrap_or_default)
    .map_err(serde::de::Error::custom)
}

fn deserialize_comma_separated_optional<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.map(|s| s.split(',').map(|s| s.to_string()).collect())
        .unwrap_or_default())
}

fn deserialize_comma_separated<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.split(',').map(|s| s.to_string()).collect())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TunneledRequest {
    pub id: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: String,
}

#[derive(Deserialize, Debug)]
pub struct TunneledHttpResponse {
    pub id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}
