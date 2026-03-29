mod config;
mod error;

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Router, routing::get};

use crate::config::LineupConfig;
use crate::error::LineupError;

#[tokio::main]
pub async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    if let Err(err) = run().await {
        log::error!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), LineupError> {
    let config_path = std::env::args()
        .nth(1)
        .ok_or(LineupError::LineupConfigRequired)?;

    // load lineup config
    let lineup_config = config::from_file(&config_path)?;

    let app = Router::new()
        .route("/", get(root))
        .route("/channels/{number}", get(stream))
        .with_state(Arc::new(lineup_config));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8409").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> String {
    String::from("ErsatzTV")
}

async fn stream(
    Path(number): Path<String>,
    State(config): State<Arc<LineupConfig>>,
) -> Result<impl IntoResponse, LineupError> {
    let channel_config = config
        .channels
        .iter()
        .find(|c| c.number == number)
        .ok_or(LineupError::ChannelNotFound(number))?;

    Ok(format!(
        "Channel {} is configured at {}",
        channel_config.number, channel_config.config
    ))
}
