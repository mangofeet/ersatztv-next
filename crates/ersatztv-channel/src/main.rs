mod channel_session;
mod config;
mod error;
mod playout_loader;

use clap::Parser;

use crate::channel_session::ChannelSession;
use crate::config::ChannelConfig;
use crate::error::ChannelError;
use crate::playout_loader::PlayoutLoader;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    config_path: std::path::PathBuf,
    #[arg(short, long)]
    output_folder: std::path::PathBuf,
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
    let playout_loader = PlayoutLoader::new(&channel_config);

    // start channel session
    let channel_session = ChannelSession::new(channel_config, playout_loader)?;
    channel_session.run().await
}
