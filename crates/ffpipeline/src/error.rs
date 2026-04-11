use thiserror::Error;

#[derive(Error, Debug)]
pub enum FFPipelineError {
    #[error("ffprobe failed")]
    ProbeFailed,
    #[error("failed to parse ffprobe output")]
    ProbeFailedToParse,
    #[error("audio input is required")]
    AudioInputIsRequired,
    #[error("video input is required")]
    VideoInputIsRequired,
}
