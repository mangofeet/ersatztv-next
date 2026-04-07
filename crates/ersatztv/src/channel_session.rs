use std::path::PathBuf;

use ersatztv_core::{READY_FILE_NAME, READY_FILE_TIMEOUT, wait_for_file};
use tokio::process::Child;
use tokio::sync::watch;

use crate::channel_model::ChannelModel;
use crate::error::LineupError;

pub struct ChannelSession {
    _output_folder: PathBuf,
    _child: Child,
    ready_receiver: watch::Receiver<bool>,
}

impl ChannelSession {
    pub fn spawn(channel: &ChannelModel) -> Result<Self, LineupError> {
        let child = tokio::process::Command::new(channel_binary_path()?)
            .arg("--output-folder")
            .arg(channel.output_folder())
            .arg(channel.config_path())
            .spawn()
            .map_err(LineupError::Io)?;

        let (ready_sender, ready_receiver) = watch::channel(false);
        let ready_file = channel.output_folder().join(READY_FILE_NAME);

        tokio::spawn(async move {
            if wait_for_file(&ready_file, READY_FILE_TIMEOUT).await {
                let _ = ready_sender.send(true);
            }
        });

        Ok(ChannelSession {
            _output_folder: channel.output_folder().to_owned(),
            _child: child,
            ready_receiver,
        })
    }

    pub fn subscribe_ready(&self) -> watch::Receiver<bool> {
        self.ready_receiver.clone()
    }
}

fn channel_binary_path() -> Result<PathBuf, LineupError> {
    let mut path = std::env::current_exe()?
        .parent()
        .ok_or(LineupError::ChannelNotFound(String::from(
            "unable to locate channel binary",
        )))?
        .to_path_buf();
    path.push("ersatztv-channel");
    Ok(path)
}
