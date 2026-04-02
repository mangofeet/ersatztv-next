use std::path::PathBuf;
use std::time::Duration;

use ersatztv_core::{READY_FILE_NAME, wait_for_file};
use tokio::process::Child;
use tokio::sync::watch;

use crate::error::LineupError;
use crate::{ChannelModel, channel_binary_path};

pub struct ChannelSession {
    _output_folder: PathBuf,
    _child: Child,
    multi_variant: String,
    ready_receiver: watch::Receiver<bool>,
}

impl ChannelSession {
    pub fn new(channel: &ChannelModel) -> Result<Self, LineupError> {
        let child = tokio::process::Command::new(channel_binary_path()?)
            .arg("--output-folder")
            .arg(&channel.output_folder)
            .arg(&channel.config)
            .spawn()
            .map_err(LineupError::Io)?;

        // not actually multi-variant, this is the variant playlist
        let multi_variant = format!("/session/{}/live.m3u8", &channel.number);

        let (ready_sender, ready_receiver) = watch::channel(false);
        let ready_file = channel.output_folder.join(READY_FILE_NAME);

        tokio::spawn(async move {
            if wait_for_file(&ready_file, Duration::from_secs(10)).await {
                let _ = ready_sender.send(true);
            }
        });

        Ok(ChannelSession {
            _output_folder: channel.output_folder.clone(),
            _child: child,
            multi_variant,
            ready_receiver,
        })
    }

    pub fn ready(&self) -> watch::Receiver<bool> {
        self.ready_receiver.clone()
    }

    pub fn entry(&self) -> &str {
        &self.multi_variant
    }
}
