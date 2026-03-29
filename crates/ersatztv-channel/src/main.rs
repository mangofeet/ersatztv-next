mod config;
mod error;

use ersatztv_playout::playout::{PlayoutItem, PlayoutItemSource};
use ffpipeline::{pipeline, probe};

use crate::config::ChannelConfig;
use crate::error::ChannelError;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    if let Err(err) = run() {
        log::error!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ChannelError> {
    // get channel config path
    let config_path = std::env::args()
        .nth(1)
        .ok_or(ChannelError::ChannelConfigRequired)?;

    // load channel config
    let channel_config = config::from_file(&config_path)?;

    // find current item
    let current_item = get_current_item(&config_path, &channel_config)?;

    let current_source = current_item
        .source
        .clone()
        .ok_or(ChannelError::PlayoutJsonSingleSourceRequired)?;

    match current_source {
        PlayoutItemSource::Local { path } => {
            // probe current item
            let probe_result = probe::probe(&path)?;
            log::debug!("probe result: {probe_result}");

            let output_folder = std::path::Path::new(&channel_config.output.folder);
            let output_file = output_folder
                .join("live.m3u8")
                .into_os_string()
                .into_string()
                .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;

            if output_folder.exists() {
                empty_folder(output_folder)
                    .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;
            } else {
                std::fs::create_dir(output_folder)
                    .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;
            }

            // generate pipeline
            let pipeline_result = pipeline::generate_pipeline(probe_result, output_file)?;
            log::debug!("pipeline result: {pipeline_result}");

            // stream current item
            let ffmpeg_output = std::process::Command::new("ffmpeg")
                .args(pipeline_result.args())
                .output()
                .map_err(|_| ChannelError::StreamFailure)?;

            if !ffmpeg_output.status.success() {
                return Err(ChannelError::StreamFailure);
            }

            Ok(())
        }
        _ => Err(ChannelError::PlayoutJsonLocalSourceRequired),
    }
}

fn get_current_item(
    config_path: &str,
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
    let entries = std::fs::read_dir(playout_folder)
        .map_err(|e| ChannelError::ChannelConfigFailure(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| ChannelError::ChannelConfigFailure(e.to_string()))?;
        let path = entry
            .path()
            .into_os_string()
            .into_string()
            .map_err(|_| ChannelError::ChannelConfigFailure(String::from("os string error")))?;
        if path.ends_with(".json") {
            log::debug!("playout JSON is {path}");

            // load playout JSON
            let playout_result = ersatztv_playout::playout::from_file(&path)?;

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

fn empty_folder(output_folder: &std::path::Path) -> Result<(), std::io::Error> {
    let entries = std::fs::read_dir(output_folder)?;
    for entry in entries {
        let entry = entry?;
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                empty_folder(&entry.path())?;
                std::fs::remove_dir(entry.path())?;
            } else {
                std::fs::remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}
