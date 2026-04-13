use std::path::PathBuf;

use serde::Deserialize;
use simple_expand_tilde::expand_tilde;
use time::OffsetDateTime;

use crate::error::ChannelError;

#[derive(Deserialize, Clone)]
pub struct ChannelConfig {
    pub playout: PlayoutConfig,
    pub normalization: NormalizationConfig,

    #[serde(skip)]
    expanded_playout_folder: PathBuf,

    #[serde(skip)]
    expanded_output_folder: PathBuf,

    #[serde(skip)]
    number: String,
}

#[derive(Deserialize, Clone)]
pub struct PlayoutConfig {
    pub folder: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub virtual_start: Option<OffsetDateTime>,
}

#[derive(Deserialize, Clone)]
pub struct NormalizationConfig {
    pub audio: AudioNormalizationConfig,
    pub video: VideoNormalizationConfig,
}

#[derive(Deserialize, Clone)]
pub struct AudioNormalizationConfig {
    pub format: Option<AudioFormat>,
    pub bitrate_kbps: Option<u32>,
    pub buffer_kbps: Option<u32>,
    pub channels: Option<u32>,
}

#[derive(Deserialize, Clone)]
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

#[derive(Deserialize, Clone)]
pub struct VideoNormalizationConfig {
    pub format: Option<VideoFormat>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bitrate_kbps: Option<u32>,
    pub buffer_kbps: Option<u32>,
    pub accel: Option<HardwareAccel>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum VideoFormat {
    H264,
    Hevc,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HardwareAccel {
    Cuda,
    Qsv,
    VideoToolbox,
}

impl From<HardwareAccel> for ffpipeline::pipeline::HardwareAccel {
    fn from(value: HardwareAccel) -> Self {
        match value {
            HardwareAccel::Cuda => ffpipeline::pipeline::HardwareAccel::Cuda,
            HardwareAccel::Qsv => ffpipeline::pipeline::HardwareAccel::Qsv,
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

impl ChannelConfig {
    pub async fn from_file(
        path: &PathBuf,
        output_folder: &PathBuf,
        number: &str,
    ) -> Result<ChannelConfig, ChannelError> {
        // load and deserialize
        let config_string = tokio::fs::read_to_string(path)
            .await
            .map_err(ChannelError::ChannelConfigIoFailure)?;
        let mut channel_config: ChannelConfig = toml::from_str(&config_string)
            .map_err(|e| ChannelError::ChannelConfigFailure(e.to_string()))?;

        // expand playout folder
        let playout_folder = PathBuf::from(&channel_config.playout.folder);
        let mut expanded_playout_folder =
            expand_tilde(&playout_folder).ok_or(ChannelError::ChannelConfigExpandPlayoutFolder)?;
        if expanded_playout_folder.is_relative() {
            let parent = path
                .parent()
                .ok_or(ChannelError::ChannelConfigFailure(String::from(
                    "failed to find parent of config",
                )))?;
            expanded_playout_folder = parent.join(&expanded_playout_folder).canonicalize()?;
        }
        channel_config.expanded_playout_folder = expanded_playout_folder;

        // expand output folder
        channel_config.expanded_output_folder =
            expand_tilde(output_folder).ok_or(ChannelError::ChannelConfigExpandOutputFolder)?;

        channel_config.number = number.to_owned();

        Ok(channel_config)
    }

    pub fn expanded_playout_folder(&self) -> &PathBuf {
        &self.expanded_playout_folder
    }

    pub fn expanded_output_folder(&self) -> &PathBuf {
        &self.expanded_output_folder
    }

    pub fn number(&self) -> &str {
        &self.number
    }
}
