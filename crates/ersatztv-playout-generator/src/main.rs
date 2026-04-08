mod error;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;
use ersatztv_playout::playout::{Playout, PlayoutItem};
use ffpipeline::probe::ProbeResult;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::DirEntry;
use walkdir::WalkDir;

use crate::error::PlayoutGeneratorError;

static VIDEO_EXTENSIONS: &[&str] = &["mkv", "mov"];

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

    // generate output file name
    let start = OffsetDateTime::now_local()?.truncate_to_day();
    let formatted_start = start.format(&Rfc3339)?;
    let finish = start + time::Duration::days(1) - time::Duration::seconds(1);
    let formatted_finish = finish.format(&Rfc3339)?;
    let output_file = format!("{formatted_start}_{formatted_finish}.json");
    log::debug!("output_file: {output_file}");

    let mut playout_items: Vec<PlayoutItem> = Vec::new();

    // fill output file with content
    let mut current_time = start;
    let mut last_index = 0;
    while current_time < finish {
        let mut index: usize = 0;
        while video_paths.len() > 1 && index == last_index {
            index = (rand::random::<u32>() as usize) % video_paths.len();
        }
        last_index = index;

        if let Some((path, probe_result)) = video_paths.iter().nth(index)
            && let Some(scheduled_duration) = probe_result.duration
            && let Ok(playout_item) = PlayoutItem::new(
                uuid::Uuid::new_v4().to_string(),
                current_time,
                path,
                scheduled_duration,
            )
        {
            playout_items.push(playout_item);
            current_time += scheduled_duration;
        }
    }

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
        && probe_result
            .duration
            .filter(|d| d.as_secs() < 120)
            .is_some()
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
