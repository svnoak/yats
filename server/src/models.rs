use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ClientParams {
    pub client_id: String,
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
