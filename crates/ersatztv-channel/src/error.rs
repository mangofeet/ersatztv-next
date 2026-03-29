use ersatztv_playout::error::PlayoutError;
use ffpipeline::error::FFPipelineError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("channel config is required as arg")]
    ChannelConfigRequired,

    #[error("unable to load channel config: {0}")]
    ChannelConfigFailure(String),

    #[error("unable to load channel config: {0}")]
    ChannelConfigIoFailure(#[from] std::io::Error),

    #[error("channel config output folder is required")]
    ChannelConfigOutputFolderRequired,

    #[error("{0}")]
    PlayoutJsonLoadFailure(#[from] PlayoutError),

    #[error("unable to find current item in playout JSON")]
    PlayoutJsonNoItem,

    #[error("only single sources are supported as playout items")]
    PlayoutJsonSingleSourceRequired,

    #[error("only local sources are supported as playout items")]
    PlayoutJsonLocalSourceRequired,

    #[error("{0}")]
    PipelineError(#[from] FFPipelineError),

    #[error("stream failed")]
    StreamFailure,
}
