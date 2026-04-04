use crate::pipeline::{AudioFormat, Kbps, VideoFormat};

#[derive(Debug)]
pub struct OutputSettings {
    pub audio_format: Option<AudioFormat>,
    pub audio_bitrate: Option<Kbps>,
    pub video_format: Option<VideoFormat>,
    pub video_bitrate: Option<Kbps>,
}

impl OutputSettings {
    pub fn new(
        audio_format: Option<AudioFormat>,
        audio_bitrate: Option<Kbps>,
        video_format: Option<VideoFormat>,
        video_bitrate: Option<Kbps>,
    ) -> Self {
        OutputSettings {
            audio_format,
            audio_bitrate,
            video_format,
            video_bitrate,
        }
    }
}
