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
    pub audio: AudioNormalizationConfig,
    pub video: VideoNormalizationConfig,
}

#[derive(Deserialize)]
pub struct AudioNormalizationConfig {
    pub format: Option<AudioFormat>,
    pub bitrate_kbps: Option<u32>,
    pub buffer_kbps: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Aac,
    Ac3,
}

impl From<AudioFormat> for ffpipeline::pipeline::AudioFormat {
    fn from(value: AudioFormat) -> Self {
        match value {
            AudioFormat::Aac => ffpipeline::pipeline::AudioFormat::Aac,
            AudioFormat::Ac3 => ffpipeline::pipeline::AudioFormat::Ac3,
        }
    }
}

#[derive(Deserialize)]
pub struct VideoNormalizationConfig {
    pub format: Option<VideoFormat>,
    pub bitrate_kbps: Option<u32>,
    pub buffer_kbps: Option<u32>,
    pub accel: Option<HardwareAccel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoFormat {
    H264,
    Hevc,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HardwareAccel {
    VideoToolbox,
}

impl From<HardwareAccel> for ffpipeline::pipeline::HardwareAccel {
    fn from(value: HardwareAccel) -> Self {
        match value {
            HardwareAccel::VideoToolbox => ffpipeline::pipeline::HardwareAccel::VideoToolbox,
        }
    }
}

impl From<VideoFormat> for ffpipeline::pipeline::VideoFormat {
    fn from(value: VideoFormat) -> Self {
        match value {
            VideoFormat::H264 => ffpipeline::pipeline::VideoFormat::H264,
            VideoFormat::Hevc => ffpipeline::pipeline::VideoFormat::Hevc,
        }
    }
}

pub async fn from_file(path: &std::path::PathBuf) -> Result<ChannelConfig, ChannelError> {
    let config_string = tokio::fs::read_to_string(path)
        .await
        .map_err(ChannelError::ChannelConfigIoFailure)?;
    let channel_config: ChannelConfig = toml::from_str(&config_string)
        .map_err(|e| ChannelError::ChannelConfigFailure(e.to_string()))?;
    Ok(channel_config)
}
