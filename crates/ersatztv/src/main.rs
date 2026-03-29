mod config;
mod error;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use ersatztv_core::{READY_FILE_NAME, empty_folder};
use tokio::process::Child;
use tokio::signal;
use tokio::sync::Mutex;

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
    let config_path = std::env::args()
        .nth(1)
        .ok_or(LineupError::LineupConfigRequired)?;

    // load lineup config
    let lineup_config = config::from_file(&config_path)?;

    let mut channels: Vec<ChannelModel> = Vec::with_capacity(lineup_config.channels.len());
    for channel in lineup_config.channels {
        match validate_channel(&config_path, &lineup_config.output.folder, channel) {
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
        .route("/channel/{number}", get(stream))
        .nest_service(
            "/session",
            tower_http::services::ServeDir::new(&lineup_config.output.folder),
        )
        .layer(axum::middleware::from_fn(fix_content_types))
        .with_state(Arc::new(state));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn stream(
    Path(number): Path<String>,
    State(state): State<Arc<LineupState>>,
) -> Result<impl IntoResponse, LineupError> {
    let channel = state
        .channels
        .iter()
        .find(|c| c.number == number)
        .ok_or(LineupError::ChannelNotFound(number.clone()))?;

    let mut active = state.active.lock().await;

    if let Some(proc) = active.get(&number) {
        return Ok(axum::response::Redirect::temporary(&proc.multi_variant));
    }

    let child = tokio::process::Command::new(channel_binary_path()?)
        .arg("--output-folder")
        .arg(&channel.output_folder)
        .arg(&channel.config)
        .spawn()
        .map_err(LineupError::Io)?;

    // not actually multi-variant, this is the variant playlist
    let multi_variant = format!("/session/{number}/live.m3u8");
    active.insert(
        number,
        ChannelProcess {
            _child: child,
            multi_variant: multi_variant.clone(),
        },
    );

    let ready_file = channel.output_folder.join(READY_FILE_NAME);
    if !wait_for_ready(&ready_file, Duration::from_secs(10)).await {
        Err(LineupError::ChannelNotFound(String::from(
            "channel timeout",
        )))
    } else {
        Ok(axum::response::Redirect::temporary(&multi_variant))
    }
}

struct ChannelModel {
    number: String,
    config: std::path::PathBuf,
    output_folder: std::path::PathBuf,
}

struct ChannelProcess {
    _child: Child,
    multi_variant: String,
}

struct LineupState {
    channels: Vec<ChannelModel>,
    active: Mutex<HashMap<String, ChannelProcess>>,
}

fn validate_channel(
    config_path: &str,
    output_folder: &str,
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

    let mut output_folder = std::path::PathBuf::from(output_folder);
    output_folder = output_folder.join(&channel.number);

    Ok(ChannelModel {
        number: channel.number,
        config: channel_config,
        output_folder,
    })
}

fn channel_binary_path() -> Result<std::path::PathBuf, LineupError> {
    let mut path = std::env::current_exe()?
        .parent()
        .ok_or(LineupError::ChannelNotFound(String::from(
            "unable to locate channel binary",
        )))?
        .to_path_buf();
    path.push("ersatztv-channel");
    Ok(path)
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

async fn wait_for_ready(path: &std::path::Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if path.exists() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(200)).await
    }
}
