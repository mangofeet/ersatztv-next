mod channel_model;
mod channel_session;
mod config;
mod error;

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use clap::Parser;
use ersatztv_core::empty_folder;
use tokio::signal;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use crate::channel_model::ChannelModel;
use crate::channel_session::ChannelSession;
use crate::error::LineupError;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    config_path: std::path::PathBuf,
}

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
        signal::ctrl_c()
            .await
            .expect("failed to install ctrl+c handler");
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
    let args = Args::parse();

    // load lineup config
    let lineup_config = config::from_file(&args.config_path).await?;

    let mut channels: Vec<ChannelModel> = Vec::with_capacity(lineup_config.channels.len());
    for channel in lineup_config.channels {
        match ChannelModel::new(&args.config_path, &lineup_config.output.folder, channel) {
            Ok(channel_config) => {
                channels.push(channel_config);
            }
            Err(err) => {
                log::error!("{err}")
            }
        }
    }

    log::debug!("loaded {} channel definitions", channels.len());

    empty_folder(std::path::Path::new(&lineup_config.output.folder)).await?;

    let state = LineupState {
        channels,
        active: Mutex::new(HashMap::new()),
    };

    let addr = format!(
        "{}:{}",
        lineup_config.server.bind_address, lineup_config.server.port
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;

    let app = Router::new()
        .route("/channel/{filename}", get(stream))
        .nest_service(
            "/session",
            tower_http::services::ServeDir::new(&lineup_config.output.folder),
        )
        .layer(axum::middleware::from_fn(fix_content_types))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn stream(
    Path(filename): Path<String>,
    State(state): State<Arc<LineupState>>,
    request: axum::extract::Request,
) -> Result<impl IntoResponse, LineupError> {
    let number = filename
        .strip_suffix(".m3u8")
        .ok_or(LineupError::ChannelNotFound(filename.clone()))?;

    let channel = state
        .channels
        .iter()
        .find(|c| c.number() == number)
        .ok_or(LineupError::ChannelNotFound(number.to_owned()))?;

    let mut ready_receiver = {
        let mut active = state.active.lock().await;

        if let Some(channel_session) = active.get(number) {
            channel_session.subscribe_ready()
        } else {
            let channel_session = ChannelSession::spawn(channel)?;
            let ready_receiver = channel_session.subscribe_ready();
            active.insert(number.to_owned(), channel_session);
            ready_receiver
        }
    };

    ready_receiver
        .wait_for(|&ready| ready)
        .await
        .map_err(|_| LineupError::ChannelNotFound(String::from("channel timeout")))?;

    // TODO: need scheme, host from reverse proxy
    let host = request
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");

    let content =
        get_multi_variant(channel).replace("/session/", &format!("http://{host}/session/"));

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/vnd.apple.mpegurl",
        )],
        content,
    ))
}

struct LineupState {
    channels: Vec<ChannelModel>,
    active: Mutex<HashMap<String, ChannelSession>>,
}

async fn fix_content_types(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let is_m3u8 = request.uri().path().ends_with(".m3u8");
    let mut response = next.run(request).await;
    if is_m3u8 && let Ok(value) = "application/vnd.apple.mpegurl".parse() {
        response
            .headers_mut()
            .insert(axum::http::header::CONTENT_TYPE, value);
    }
    response
}

fn get_multi_variant(channel: &ChannelModel) -> String {
    format!(
        "#EXTM3U
#EXT-X-VERSION:3
#EXT-X-STREAM-INF:BANDWIDTH=5000000
/session/{}/live.m3u8",
        channel.number()
    )
}
