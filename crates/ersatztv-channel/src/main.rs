mod config;
mod error;

use std::time::Duration;

use clap::Parser;
use ersatztv_core::{READY_FILE_NAME, empty_folder, wait_for_file};
use ersatztv_playout::playout::{PlayoutItem, PlayoutItemSource};
use ffpipeline::{pipeline, probe};

use crate::config::ChannelConfig;
use crate::error::ChannelError;

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
    let channel_config = config::from_file(&args.config_path).await?;

    // find current item
    let current_item = get_current_item(&args.config_path, &channel_config).await?;

    let current_source = current_item
        .source
        .clone()
        .ok_or(ChannelError::PlayoutJsonSingleSourceRequired)?;

    match current_source {
        PlayoutItemSource::Local { path } => {
            // probe current item
            let probe_result = probe::probe(&path)?;
            log::debug!("probe result: {probe_result}");

            let output_folder = std::path::Path::new(&args.output_folder);
            let output_file = output_folder
                .join("live.m3u8")
                .into_os_string()
                .into_string()
                .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;

            let ready_file = output_folder.join(READY_FILE_NAME);
            if ready_file.exists() {
                tokio::fs::remove_file(&ready_file).await?;
            }

            if output_folder.exists() {
                empty_folder(output_folder)
                    .await
                    .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;
            } else {
                tokio::fs::create_dir(output_folder)
                    .await
                    .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;
            }

            // generate pipeline
            let pipeline_result = pipeline::generate_pipeline(probe_result, output_file)?;
            log::debug!("pipeline result: {pipeline_result}");

            // stream current item
            let mut ffmpeg_child = tokio::process::Command::new("ffmpeg")
                .args(pipeline_result.args())
                .spawn()
                .map_err(|_| ChannelError::StreamFailure(String::from("failed to spawn ffmpeg")))?;

            let ready = tokio::select! {
                status = ffmpeg_child.wait() => {
                    let status = status.map_err(|_| ChannelError::StreamFailure(String::from("ffmpeg exit code")))?;
                    if !status.success() {
                        return Err(ChannelError::StreamFailure(String::from("ffmpeg exit code")));
                    }

                    true
                }

                // wait for segment #4 to exist
                result = async {
                    let target_file = output_folder.join("live3.ts");
                    return wait_for_file(&target_file, Duration::from_secs(30)).await;
                } => {
                    result
                }
            };

            if ready {
                tokio::fs::write(&ready_file, b"").await?;
                Ok(())
            } else {
                Err(ChannelError::StreamFailure(String::from(
                    "not ready in time",
                )))
            }
        }
        _ => Err(ChannelError::PlayoutJsonLocalSourceRequired),
    }
}

async fn get_current_item(
    config_path: &std::path::PathBuf,
    channel_config: &ChannelConfig,
) -> Result<PlayoutItem, ChannelError> {
    // TODO: better algorithm for finding appropriate playout JSON file

    let mut playout_folder = std::path::PathBuf::from(&channel_config.playout.folder);
    if playout_folder.is_relative() {
        let parent = std::path::Path::new(config_path).parent().ok_or(
            ChannelError::ChannelConfigFailure(String::from("failed to find parent of config")),
        )?;
        playout_folder = parent.join(&playout_folder).canonicalize()?;
    }

    log::debug!("playout folder is {}", playout_folder.to_string_lossy());

    // find first playout JSON in folder
    let mut entries = tokio::fs::read_dir(playout_folder)
        .await
        .map_err(|e| ChannelError::ChannelConfigFailure(e.to_string()))?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry
            .path()
            .into_os_string()
            .into_string()
            .map_err(|_| ChannelError::ChannelConfigFailure(String::from("os string error")))?;
        if path.ends_with(".json") {
            log::debug!("playout JSON is {path}");

            // load playout JSON
            let playout_result = ersatztv_playout::playout::from_file(&path).await?;

            // find current item
            return playout_result
                .playout
                .items
                .into_iter()
                .next()
                .ok_or(ChannelError::PlayoutJsonNoItem);
        }
    }

    Err(ChannelError::ChannelConfigFailure(String::from(
        "found no files",
    )))
}
