use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct TunneledHttpRequest {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TunneledHttpResponse {
    pub request_id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}