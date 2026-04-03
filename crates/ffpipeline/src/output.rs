#[derive(Debug)]
pub struct OutputSettings {
    pub video_format: String,
}

impl OutputSettings {
    pub fn new(video_format: String) -> Self {
        OutputSettings { video_format }
    }
}
