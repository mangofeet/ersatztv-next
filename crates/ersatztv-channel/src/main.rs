mod config;
mod error;
mod playout_loader;

use std::time::Duration;

use clap::Parser;
use ersatztv_core::{READY_FILE_NAME, READY_FILE_TIMEOUT, empty_folder, wait_for_file};
use ersatztv_playout::playout::PlayoutItemSource;
use ffpipeline::input::{InputSettings, ProbedInput};
use ffpipeline::output::OutputSettings;
use ffpipeline::pipeline::{AudioFormat, HardwareAccel, Kbps, VideoFormat};
use ffpipeline::{pipeline, probe};
use simple_expand_tilde::expand_tilde;
use time::OffsetDateTime;

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
    let channel_config = ChannelConfig::from_file(&args.config_path).await?;
    let playout_loader = PlayoutLoader::new(&channel_config);

    // find current item
    let now = OffsetDateTime::now_local()?;
    let current_item = playout_loader.get_current_item(&now).await?;
    let finish = current_item.start + Duration::from_millis(current_item.duration_ms);
    log::debug!(
        "current playout item starts at {} and finishes at {}",
        current_item.start,
        finish
    );

    let current_source = current_item
        .source
        .clone()
        .ok_or(ChannelError::PlayoutJsonSingleSourceRequired)?;

    match current_source {
        PlayoutItemSource::Local { path } => {
            let expanded_path_buf =
                expand_tilde(&path).ok_or(ChannelError::PlayoutJsonInvalidLocalSource)?;
            let expanded_path = expanded_path_buf
                .as_os_str()
                .to_str()
                .ok_or(ChannelError::PlayoutJsonInvalidLocalSource)?;

            // probe current item
            let probe_result = probe::probe(expanded_path)?;
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
            let output_settings = OutputSettings {
                audio_format: channel_config
                    .normalization
                    .audio
                    .format
                    .map(AudioFormat::from),
                audio_bitrate: channel_config.normalization.audio.bitrate_kbps.map(Kbps),
                audio_buffer: channel_config.normalization.audio.buffer_kbps.map(Kbps),
                video_format: channel_config
                    .normalization
                    .video
                    .format
                    .map(VideoFormat::from),
                video_bitrate: channel_config.normalization.video.bitrate_kbps.map(Kbps),
                video_buffer: channel_config.normalization.video.buffer_kbps.map(Kbps),
                accel: channel_config
                    .normalization
                    .video
                    .accel
                    .map(HardwareAccel::from),
                format: pipeline::OutputFormat::Hls(output_file),
            };
            let in_point =
                Duration::from_millis((now - current_item.start).whole_milliseconds() as u64);
            let out_point = in_point + Duration::from_millis(current_item.duration_ms);

            let input_settings = InputSettings {
                input: ProbedInput {
                    in_point,
                    out_point,
                    probe_result,
                },
            };

            let pipeline_result = pipeline::generate_pipeline(input_settings, output_settings)?;
            log::debug!("pipeline result: {pipeline_result}");

            // stream current item
            let mut ffmpeg_child = tokio::process::Command::new("ffmpeg")
                .args(pipeline_result.args())
                .spawn()
                .map_err(|_| ChannelError::StreamFailure(String::from("failed to spawn ffmpeg")))?;

            let (ready, ffmpeg_already_exited) = tokio::select! {
                status = ffmpeg_child.wait() => {
                    let status = status.map_err(|_| ChannelError::StreamFailure(String::from("ffmpeg exit code")))?;
                    if !status.success() {
                        return Err(ChannelError::StreamFailure(String::from("ffmpeg exit code")));
                    }

                    (true, true)
                }

                // wait for segment #4 to exist
                result = async {
                    let target_file = output_folder.join("live3.ts");
                    return wait_for_file(&target_file, READY_FILE_TIMEOUT).await;
                } => {
                    (result, false)
                }
            };

            if ready {
                tokio::fs::write(&ready_file, b"").await?;

                if !ffmpeg_already_exited {
                    log::debug!("waiting for ffmpeg to terminate...");
                    let status = ffmpeg_child
                        .wait()
                        .await
                        .map_err(|e| ChannelError::StreamFailure(e.to_string()))?;
                    log::debug!("ffmpeg exited with status: {status}");
                    if !status.success() {
                        return Err(ChannelError::StreamFailure(format!(
                            "ffmoeg exited: {status}"
                        )));
                    }
                }

                Ok(())
            } else {
                ffmpeg_child.kill().await.ok();
                Err(ChannelError::StreamFailure(String::from(
                    "not ready in time",
                )))
            }
        }
        _ => Err(ChannelError::PlayoutJsonLocalSourceRequired),
    }
}
