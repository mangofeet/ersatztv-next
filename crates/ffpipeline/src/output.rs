use crate::pipeline::Kbps;

#[derive(Debug)]
pub struct OutputSettings {
    pub video_format: String,
    pub video_bitrate: Option<Kbps>,
}

impl OutputSettings {
    pub fn new(video_format: String, video_bitrate: Option<Kbps>) -> Self {
        OutputSettings {
            video_format,
            video_bitrate,
        }
    }
}
