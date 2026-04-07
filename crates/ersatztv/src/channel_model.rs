use std::path::{Path, PathBuf};

use crate::config::ChannelConfig;
use crate::error::LineupError;

pub struct ChannelModel {
    number: String,
    config_path: PathBuf,
    output_folder: PathBuf,
}

impl ChannelModel {
    pub fn new(
        config_path: &Path,
        output_folder: &str,
        channel: ChannelConfig,
    ) -> Result<ChannelModel, LineupError> {
        let mut channel_config = PathBuf::from(&channel.config);
        if channel_config.is_relative() {
            let parent = config_path
                .parent()
                .ok_or(LineupError::LineupConfigFailure(String::from(
                    "failed to find parent of config",
                )))?;
            channel_config = parent.join(&channel_config).canonicalize()?;
        }

        let output_folder = PathBuf::from(output_folder);

        Ok(ChannelModel {
            number: channel.number.clone(),
            config_path: channel_config,
            output_folder: output_folder.join(&channel.number),
        })
    }

    pub fn number(&self) -> &str {
        self.number.as_str()
    }

    pub fn config_path(&self) -> &Path {
        self.config_path.as_path()
    }

    pub fn output_folder(&self) -> &Path {
        self.output_folder.as_path()
    }
}
