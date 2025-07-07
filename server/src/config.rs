use dotenvy::dotenv;
use std::env;

pub struct Config {
    pub secret_token: String,
    pub is_production: bool,
}

impl Config {
    pub fn new() -> Self {
        dotenv().ok();
        let secret_token = env::var("SECRET_TOKEN").expect("SECRET_TOKEN must be set");
        let is_production = env::var("IS_PRODUCTION")
            .map(|val| val == "true")
            .unwrap_or(false);
        Self {
            secret_token,
            is_production,
        }
    }
}
