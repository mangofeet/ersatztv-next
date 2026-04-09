mod channel_session;
mod config;
mod error;
mod playlist_manager;
mod playout_loader;
mod pts_scanner;

use clap::Parser;
use ersatztv_core::{READY_FILE_TIMEOUT, wait_for_file};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::channel_session::ChannelSession;
use crate::config::ChannelConfig;
use crate::error::ChannelError;

const PLAYLIST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    config_path: PathBuf,
    #[arg(short, long)]
    output_folder: PathBuf,
}

#[tokio::main]
pub async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    if let Err(err) = run().await {
        log::error!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), ChannelError> {
    let args = Args::parse();

    // load channel config
    let channel_config = ChannelConfig::from_file(&args.config_path, &args.output_folder).await?;

    // start channel session
    let mut channel_session = ChannelSession::new(channel_config)?;

    let output_file = channel_session.output_file().to_owned();
    let output_folder = channel_session.output_folder().to_owned();
    let ready_file = channel_session.ready_file().to_owned();

    let (fail_tx, fail_rx) = tokio::sync::oneshot::channel::<ChannelError>();

    tokio::spawn(async move {
        match wait_for_segments(&output_file, &output_folder, &ready_file).await {
            Ok(()) => { /* do nothing */ }
            Err(e) => {
                let _ = fail_tx.send(e);
            }
        }
    });

    tokio::select! {
        result = channel_session.run() => result,

        // only cancel session run when segment wait fails
        Ok(err) = fail_rx => Err(err),
    }
}

async fn wait_for_segments(
    output_file: &str,
    output_folder: &Path,
    ready_file: &PathBuf,
) -> Result<(), ChannelError> {
    // first wait for playlist to exist
    let playlist_path = Path::new(output_file);
    let playlist_exists = wait_for_file(playlist_path, PLAYLIST_TIMEOUT).await;
    if !playlist_exists {
        return Err(ChannelError::StreamFailure(String::from(
            "timeout waiting for initial playlist",
        )));
    }

    // then wait for segments
    let target_file = output_folder.join("live000003.ts");
    let ready = wait_for_file(&target_file, READY_FILE_TIMEOUT).await;
    if !ready {
        return Err(ChannelError::StreamFailure(String::from(
            "timeout waiting for initial segments",
        )));
    }

    tokio::fs::write(&ready_file, b"").await?;
    Ok(())
}
