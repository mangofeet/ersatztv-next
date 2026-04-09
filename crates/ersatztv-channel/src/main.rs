mod channel_session;
mod config;
mod error;
mod playlist_manager;
mod playout_loader;
mod pts_scanner;

use clap::Parser;
use std::path::PathBuf;

use crate::channel_session::ChannelSession;
use crate::config::ChannelConfig;
use crate::error::ChannelError;

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
    channel_session.run().await
}
