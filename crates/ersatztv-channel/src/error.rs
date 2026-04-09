use ersatztv_playout::error::PlayoutError;
use ffpipeline::error::FFPipelineError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("unable to load channel config: {0}")]
    ChannelConfigFailure(String),

    #[error("unable to load channel config (io): {0}")]
    ChannelConfigIoFailure(#[from] std::io::Error),

    #[error("failed to expand playout folder")]
    ChannelConfigExpandPlayoutFolder,

    #[error("failed to expand output folder")]
    ChannelConfigExpandOutputFolder,

    #[error("channel config output folder is required")]
    ChannelConfigOutputFolderRequired,

    #[error("date formatting error: {0}")]
    ChannelDateFormatError(#[from] time::error::Format),

    #[error("Indeterminate local time offset: {0}")]
    DateOffsetError(#[from] time::error::IndeterminateOffset),

    #[error("{0}")]
    PlayoutJsonLoadFailure(#[from] PlayoutError),

    #[error("unable to find current item in playout JSON")]
    PlayoutJsonNoItem,

    #[error("only single sources are supported as playout items")]
    PlayoutJsonSingleSourceRequired,

    #[error("only local sources are supported as playout items")]
    PlayoutJsonLocalSourceRequired,

    #[error("local source is invalid for playout item")]
    PlayoutJsonInvalidLocalSource,

    #[error("{0}")]
    PipelineError(#[from] FFPipelineError),

    #[error("stream failed: {0}")]
    StreamFailure(String),

    #[error("failed to scan for last pts")]
    PtsScannerFailure,

    #[error("channel terminated after idle timeout")]
    IdleTimeout,
}
