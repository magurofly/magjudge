use std::{fs::File, io::Read};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use toml::{from_str, Table};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let mut config_toml = String::new();
    File::open("config.toml").unwrap().read_to_string(&mut config_toml).unwrap();
    from_str(&config_toml).unwrap()
});

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub program: ProgramConfig,
    pub server: ServerConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProgramConfig {
    pub source_path: String,
    pub execute_dir: String,
    pub dependency_dir: String,
    pub time_limit: u64,
    pub externs: Table,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub addr_port: String,
    pub ssl_cert_path: String,
    pub ssl_key_path: String,
    pub public_files: Table,
    pub keep_submission_time: u64,
}
