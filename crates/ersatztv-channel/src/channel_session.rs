use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ersatztv_core::{READY_FILE_NAME, empty_folder};
use ersatztv_playout::playout::PlayoutItemSource;
use ffpipeline::input::{InputSettings, ProbedInput};
use ffpipeline::output::OutputSettings;
use ffpipeline::pipeline::{AudioFormat, HardwareAccel, Kbps, PtsOffset, VideoFormat};
use ffpipeline::{pipeline, probe};
use simple_expand_tilde::expand_tilde;
use time::OffsetDateTime;
use tokio::sync::Mutex;

use crate::config::ChannelConfig;
use crate::error::ChannelError;
use crate::playlist_manager::PlaylistManager;
use crate::playout_loader::PlayoutLoader;
use crate::pts_scanner::{PtsScanner, PtsTime};

pub struct ChannelSession {
    channel_config: ChannelConfig,
    playout_loader: PlayoutLoader,
    pts_scanner: PtsScanner,
    playlist_manager: Arc<Mutex<PlaylistManager>>,

    transcoded_until: OffsetDateTime,
    output_folder: PathBuf,
    ready_file: PathBuf,

    output_file: String,
    output_segment_template: String,
}

impl ChannelSession {
    pub fn new(channel_config: ChannelConfig) -> Result<ChannelSession, ChannelError> {
        let now = OffsetDateTime::now_local()?;

        let output_folder = channel_config.expanded_output_folder().to_owned();
        let generated_output_file = output_folder
            .join("live.m3u8")
            .into_os_string()
            .into_string()
            .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;

        let ffmpeg_output_file = output_folder
            .join("ffmpeg.m3u8")
            .into_os_string()
            .into_string()
            .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;

        let output_segment_template = output_folder
            .join("live%06d.ts")
            .into_os_string()
            .into_string()
            .map_err(|_| ChannelError::ChannelConfigOutputFolderRequired)?;

        let ready_file = output_folder.join(READY_FILE_NAME);

        let playout_loader = PlayoutLoader::new(&channel_config);
        let pts_scanner = PtsScanner::new(&channel_config);
        let playlist_manager = PlaylistManager::new(
            now,
            pipeline::SEGMENT_SECONDS,
            output_folder.to_owned(),
            generated_output_file,
            ffmpeg_output_file.to_owned(),
        );

        let playlist_manager = Arc::new(Mutex::new(playlist_manager));

        Ok(ChannelSession {
            channel_config,
            playout_loader,
            pts_scanner,
            playlist_manager,
            transcoded_until: now,
            output_folder,
            ready_file,
            output_file: ffmpeg_output_file,
            output_segment_template,
        })
    }

    pub fn output_file(&self) -> &str {
        &self.output_file
    }

    pub fn output_folder(&self) -> &PathBuf {
        &self.output_folder
    }

    pub fn ready_file(&self) -> &PathBuf {
        &self.ready_file
    }

    pub async fn run(&mut self) -> Result<(), ChannelError> {
        self.prep_output_folder().await?;

        let pm = self.playlist_manager.clone();
        tokio::spawn(async move {
            loop {
                let _ = pm.lock().await.update().await;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        self.transcode().await?;

        loop {
            let now = OffsetDateTime::now_local()?;
            let transcoded_buffer =
                std::cmp::max(time::Duration::new(0, 0), self.transcoded_until - now);
            log::debug!(
                "transcoded buffer: {}m {}s",
                transcoded_buffer.whole_minutes(),
                transcoded_buffer.whole_seconds() % 60
            );
            if transcoded_buffer <= time::Duration::minutes(1) {
                self.transcode().await?;
            } else {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    async fn prep_output_folder(&self) -> Result<(), ChannelError> {
        let output_folder = self.channel_config.expanded_output_folder();

        if self.ready_file.exists() {
            tokio::fs::remove_file(&self.ready_file).await?;
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

        Ok(())
    }

    async fn transcode(&mut self) -> Result<(), ChannelError> {
        // get last pts offset
        let mut pts_time: Option<PtsTime> = None;
        match self.pts_scanner.get_last_pts().await {
            Ok(scanned_pts_time) => pts_time = Some(scanned_pts_time),
            Err(e) => log::debug!("failed to scan pts time: {e}"),
        }

        // TODO: work ahead vs realtime

        // TODO: transcode error message instead of bailing
        let current_item = self
            .playout_loader
            .get_current_item(&self.transcoded_until)
            .await?;

        let current_source = current_item
            .source
            .clone()
            .ok_or(ChannelError::PlayoutJsonSingleSourceRequired)?;

        let probe_result = match current_source {
            PlayoutItemSource::Local { path } => {
                let expanded_path_buf =
                    expand_tilde(&path).ok_or(ChannelError::PlayoutJsonInvalidLocalSource)?;
                let expanded_path = expanded_path_buf
                    .as_os_str()
                    .to_str()
                    .ok_or(ChannelError::PlayoutJsonInvalidLocalSource)?;

                // probe current item
                Ok(probe::probe(expanded_path)?)
            }
            _ => Err(ChannelError::PlayoutJsonLocalSourceRequired),
        }?;

        log::debug!("probe result: {probe_result}");

        // generate pipeline
        let output_settings = OutputSettings {
            audio_format: self
                .channel_config
                .normalization
                .audio
                .format
                .clone()
                .map(AudioFormat::from),
            audio_bitrate: self
                .channel_config
                .normalization
                .audio
                .bitrate_kbps
                .map(Kbps),
            audio_buffer: self
                .channel_config
                .normalization
                .audio
                .buffer_kbps
                .map(Kbps),
            video_format: self
                .channel_config
                .normalization
                .video
                .format
                .clone()
                .map(VideoFormat::from),
            video_bitrate: self
                .channel_config
                .normalization
                .video
                .bitrate_kbps
                .map(Kbps),
            video_buffer: self
                .channel_config
                .normalization
                .video
                .buffer_kbps
                .map(Kbps),
            accel: self
                .channel_config
                .normalization
                .video
                .accel
                .clone()
                .map(HardwareAccel::from),
            format: pipeline::OutputFormat::Hls {
                playlist: self.output_file.clone(),
                segment_template: self.output_segment_template.clone(),
            },
            pts_offset: pts_time.map(|p| PtsOffset {
                duration: p.duration,
            }),
        };

        // in and out points from playout item
        let in_point_base_ms = current_item.in_point_ms.unwrap_or(0);
        let item_duration_ms =
            (current_item.finish - current_item.start).whole_milliseconds() as u64;
        let in_point = if self.transcoded_until > current_item.start {
            Duration::from_millis(
                in_point_base_ms
                    + (self.transcoded_until - current_item.start).whole_milliseconds() as u64,
            )
        } else {
            Duration::from_millis(in_point_base_ms)
        };
        let out_point = Duration::from_millis(
            current_item
                .out_point_ms
                .unwrap_or(in_point_base_ms + item_duration_ms),
        );

        let input_settings = InputSettings {
            input: ProbedInput {
                in_point,
                out_point,
                probe_result,
            },
        };

        let mut pipeline_result = pipeline::generate_pipeline(input_settings, output_settings)?;
        pipeline_result.optimize();
        log::debug!("optimized pipeline: {pipeline_result}");

        self.playlist_manager
            .lock()
            .await
            .before_new_pipeline()
            .await?;

        // stream current item
        let mut ffmpeg_child = tokio::process::Command::new("ffmpeg")
            .args(pipeline_result.args())
            .spawn()
            .map_err(|_| ChannelError::StreamFailure(String::from("failed to spawn ffmpeg")))?;

        log::debug!("waiting for ffmpeg to terminate...");

        let status = ffmpeg_child
            .wait()
            .await
            .map_err(|e| ChannelError::StreamFailure(e.to_string()))?;

        if !status.success() {
            return Err(ChannelError::StreamFailure(format!(
                "ffmpeg exited {status}"
            )));
        }

        self.transcoded_until = current_item.finish;
        log::debug!("transcoded until: {}", current_item.finish);

        Ok(())
    }
}
