use serde::Deserialize;

use crate::error::LineupError;

#[derive(Deserialize, Clone)]
pub struct LineupConfig {
    pub channels: Vec<ChannelConfig>,
}

#[derive(Deserialize, Clone)]
pub struct ChannelConfig {
    pub number: String,
    pub config: String,
}

pub fn from_file(path: &str) -> Result<LineupConfig, LineupError> {
    let config_string = std::fs::read_to_string(path)
        .map_err(|e| LineupError::LineupConfigFailure(e.to_string()))?;
    let lineup_config: LineupConfig = toml::from_str(&config_string)
        .map_err(|e| LineupError::LineupConfigFailure(e.to_string()))?;
    Ok(lineup_config)
}
