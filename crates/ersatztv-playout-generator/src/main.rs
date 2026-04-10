mod error;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;
use ersatztv_playout::playout::{Playout, PlayoutItem};
use ffpipeline::probe::ProbeResult;
use rand::RngExt;
use rand::seq::SliceRandom;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::DirEntry;
use walkdir::WalkDir;

use crate::error::PlayoutGeneratorError;

static VIDEO_EXTENSIONS: &[&str] = &[
    "avs", "mpg", "mp2", "mpeg", "mpe", "mpv", "ogg", "ogv", "mp4", "m4p", "m4v", "avi", "wmv",
    "mov", "mkv", "m2ts", "ts", "webm",
];

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    content_folder: PathBuf,
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

async fn run() -> Result<(), PlayoutGeneratorError> {
    let args = Args::parse();

    // find all available video files
    let mut video_paths: HashMap<PathBuf, ProbeResult> = HashMap::new();
    for path_and_probe in WalkDir::new(args.content_folder)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(is_video_extension)
        .filter_map(to_probe_result)
    {
        log::debug!("path: {path_and_probe:?}");
        video_paths.insert(path_and_probe.path, path_and_probe.probe);
    }

    if video_paths.is_empty() {
        return Err(PlayoutGeneratorError::NoSourceContent);
    }

    // generate output file name
    let start = OffsetDateTime::now_local()?.truncate_to_day();
    let formatted_start = start.format(&Rfc3339)?;
    let finish = start + time::Duration::days(1) - time::Duration::seconds(1);

    let video_list: Vec<(PathBuf, ProbeResult)> = video_paths.into_iter().collect();

    // fill output file with content
    let mut playout_items: Vec<PlayoutItem> = Vec::new();
    let mut current_time = start;
    let mut rng = rand::rng();
    let mut shuffled = video_list.clone();
    shuffled.shuffle(&mut rng);
    let mut cursor = 0;

    while current_time < finish {
        if cursor >= shuffled.len() {
            let last = shuffled.last().cloned();
            shuffled.shuffle(&mut rng);
            if shuffled.len() > 1
                && let Some(ref last) = last
                && shuffled[0].0 == last.0
            {
                let swap_index = rng.random_range(1..shuffled.len());
                shuffled.swap(0, swap_index);
            }
            cursor = 0;
        }

        let (path, probe_result) = &shuffled[cursor];
        cursor += 1;

        if let Some(scheduled_duration) = probe_result.duration
            && let Ok(playout_item) = PlayoutItem::new(
                uuid::Uuid::new_v4().to_string(),
                current_time,
                current_time + scheduled_duration,
                None,
                None,
                path,
            )
        {
            playout_items.push(playout_item);
            current_time += scheduled_duration;
        }
    }

    let formatted_finish = current_time.format(&Rfc3339)?;
    let output_file = format!("{formatted_start}_{formatted_finish}.json");
    log::debug!("output_file: {output_file}");

    if !args.output_folder.exists() {
        tokio::fs::create_dir_all(&args.output_folder).await?;
    }

    let output_path = args.output_folder.join(&output_file);
    let playout = Playout::new(playout_items);
    let output_string = serde_json::to_string(&playout)?;
    tokio::fs::write(&output_path, output_string).await?;

    Ok(())
}

fn is_video_extension(dir_entry: &DirEntry) -> bool {
    if let Some(extension) = dir_entry.path().extension()
        && let Some(extension) = extension.to_str()
    {
        return VIDEO_EXTENSIONS.contains(&extension);
    }

    false
}

fn to_probe_result(dir_entry: DirEntry) -> Option<PathAndProbe> {
    if let Some(video_path) = dir_entry.path().to_str()
        && let Ok(probe_result) = ffpipeline::probe::probe(video_path)
    // && probe_result
    //     .duration
    //     .filter(|d| d.as_secs() < 120)
    //     .is_some()
    {
        return Some(PathAndProbe {
            path: dir_entry.path().to_path_buf(),
            probe: probe_result,
        });
    }

    None
}

#[derive(Debug)]
struct PathAndProbe {
    path: PathBuf,
    probe: ProbeResult,
}
