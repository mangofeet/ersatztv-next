use crate::pipeline::{AudioFormat, HardwareAccel, Kbps, OutputFormat, VideoFormat};

#[derive(Debug)]
pub struct OutputSettings {
    pub audio_format: Option<AudioFormat>,
    pub audio_bitrate: Option<Kbps>,
    pub audio_buffer: Option<Kbps>,
    pub video_format: Option<VideoFormat>,
    pub video_bitrate: Option<Kbps>,
    pub video_buffer: Option<Kbps>,
    pub accel: Option<HardwareAccel>,
    pub format: OutputFormat,
}
