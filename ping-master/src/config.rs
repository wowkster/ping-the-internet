use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub workers: WorkerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkerConfig {
    Local(LocalWorkerConfig),
    Remote,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalWorkerConfig {
    pub count: u16,
    pub max_connections: u16,
    pub retry_limit: u16,
    pub timeout: u16,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO Error ({0})")]
    Io(#[from] std::io::Error),
    #[error("Parse Error ({0})")]
    Toml(#[from] toml::de::Error),
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)?;

    Ok(toml::from_str(&contents)?)
}
