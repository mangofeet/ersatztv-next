mod config;
mod error;

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use tokio::signal;

use crate::config::ChannelConfig;
use crate::error::LineupError;

#[tokio::main]
pub async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    if let Err(err) = run().await {
        log::error!("{err}");
        std::process::exit(1);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install ctrl+c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn run() -> Result<(), LineupError> {
    let config_path = std::env::args()
        .nth(1)
        .ok_or(LineupError::LineupConfigRequired)?;

    // load lineup config
    let lineup_config = config::from_file(&config_path)?;

    let mut channels: Vec<ChannelModel> = Vec::with_capacity(lineup_config.channels.len());
    for channel in lineup_config.channels {
        match validate_channel(&config_path, channel) {
            Ok(channel_config) => {
                channels.push(channel_config);
            }
            Err(err) => {
                log::error!("{err}")
            }
        }
    }

    let addr = format!(
        "{}:{}",
        lineup_config.server.bind_address, lineup_config.server.port
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;

    let app = Router::new()
        .route("/channels/{number}", get(stream))
        .with_state(Arc::new(channels));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn stream(
    Path(number): Path<String>,
    State(config): State<Arc<Vec<ChannelModel>>>,
) -> Result<impl IntoResponse, LineupError> {
    let channel_config = config
        .iter()
        .find(|c| c.number == number)
        .ok_or(LineupError::ChannelNotFound(number))?;

    Ok(format!(
        "Channel {} is configured at {}",
        channel_config.number,
        channel_config.config.to_string_lossy()
    ))
}

struct ChannelModel {
    number: String,
    config: std::path::PathBuf,
}

fn validate_channel(
    config_path: &str,
    channel: ChannelConfig,
) -> Result<ChannelModel, LineupError> {
    let mut channel_config = std::path::PathBuf::from(&channel.config);
    if channel_config.is_relative() {
        let parent =
            std::path::Path::new(config_path)
                .parent()
                .ok_or(LineupError::LineupConfigFailure(String::from(
                    "failed to find parent of config",
                )))?;
        channel_config = parent.join(&channel_config).canonicalize()?;
    }
    Ok(ChannelModel {
        number: channel.number,
        config: channel_config,
    })
}
