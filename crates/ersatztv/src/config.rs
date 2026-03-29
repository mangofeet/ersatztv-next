use serde::Deserialize;

use crate::error::LineupError;

#[derive(Deserialize, Clone)]
pub struct LineupConfig {
    #[serde(default = "server_config_default")]
    pub server: ServerConfig,
    pub output: OutputConfig,
    pub channels: Vec<ChannelConfig>,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "bind_address_default")]
    pub bind_address: String,
    #[serde(default = "port_default")]
    pub port: u16,
}

#[derive(Deserialize, Clone)]
pub struct OutputConfig {
    pub folder: String,
}

#[derive(Deserialize, Clone)]
pub struct ChannelConfig {
    pub number: String,
    pub config: String,
}

fn server_config_default() -> ServerConfig {
    ServerConfig {
        bind_address: bind_address_default(),
        port: port_default(),
    }
}

fn bind_address_default() -> String {
    String::from("0.0.0.0")
}
fn port_default() -> u16 {
    8409
}

pub fn from_file(path: &str) -> Result<LineupConfig, LineupError> {
    let config_string = std::fs::read_to_string(path)
        .map_err(|e| LineupError::LineupConfigFailure(e.to_string()))?;
    let lineup_config: LineupConfig = toml::from_str(&config_string)
        .map_err(|e| LineupError::LineupConfigFailure(e.to_string()))?;
    Ok(lineup_config)
}
