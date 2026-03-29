use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlayoutError {
    #[error("playout JSON does not exist")]
    PlayoutJsonDoesNotExist,

    #[error("failed to load playout JSON file: {0}")]
    PlayoutJsonLoadError(String),
}
