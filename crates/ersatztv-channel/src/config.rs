use serde::Deserialize;

use crate::error::ChannelError;

#[derive(Deserialize)]
pub struct ChannelConfig {
    pub playout: PlayoutConfig,
    pub normalization: NormalizationConfig,
}

#[derive(Deserialize)]
pub struct PlayoutConfig {
    pub folder: String,
}

#[derive(Deserialize)]
pub struct NormalizationConfig {
    pub video: VideoNormalizationConfig,
    pub audio: AudioNormalizationConfig,
}

#[derive(Deserialize)]
pub struct VideoNormalizationConfig {
    pub format: String,
    pub bitrate_kbps: Option<u32>,
}

#[derive(Deserialize)]
pub struct AudioNormalizationConfig {
    pub format: String,
    pub bitrate_kbps: Option<u32>,
}

pub async fn from_file(path: &std::path::PathBuf) -> Result<ChannelConfig, ChannelError> {
    let config_string = tokio::fs::read_to_string(path)
        .await
        .map_err(ChannelError::ChannelConfigIoFailure)?;
    let channel_config: ChannelConfig = toml::from_str(&config_string)
        .map_err(|e| ChannelError::ChannelConfigFailure(e.to_string()))?;
    Ok(channel_config)
}
