use dotenvy::dotenv;
use std::{env, path::PathBuf};

pub struct Config {
    pub secret_token: String,
    pub is_production: bool,
    pub asn_db_path: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        dotenv().ok();
        let secret_token = env::var("SECRET_TOKEN").expect("SECRET_TOKEN must be set");
        let is_production = env::var("IS_PRODUCTION")
            .map(|val| val == "true")
            .unwrap_or(false);
        let asn_db_path: PathBuf = env::var("ASN_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or(PathBuf::from("asn-test.mmdb"));
        Self {
            secret_token,
            is_production,
            asn_db_path,
        }
    }
}
