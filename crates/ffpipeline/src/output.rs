use crate::pipeline::{AudioFormat, HardwareAccel, Kbps, VideoFormat};

#[derive(Debug)]
pub struct OutputSettings {
    pub audio_format: Option<AudioFormat>,
    pub audio_bitrate: Option<Kbps>,
    pub audio_buffer: Option<Kbps>,
    pub video_format: Option<VideoFormat>,
    pub video_bitrate: Option<Kbps>,
    pub video_buffer: Option<Kbps>,
    pub accel: Option<HardwareAccel>,
}

impl OutputSettings {
    pub fn new(
        audio_format: Option<AudioFormat>,
        audio_bitrate: Option<Kbps>,
        audio_buffer: Option<Kbps>,
        video_format: Option<VideoFormat>,
        video_bitrate: Option<Kbps>,
        video_buffer: Option<Kbps>,
        accel: Option<HardwareAccel>,
    ) -> Self {
        OutputSettings {
            audio_format,
            audio_bitrate,
            audio_buffer,
            video_format,
            video_bitrate,
            video_buffer,
            accel,
        }
    }
}
