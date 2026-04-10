use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LineupError {
    #[error("io error {0}")]
    Io(#[from] std::io::Error),

    #[error("unable to load lineup config: {0}")]
    LineupConfigFailure(String),

    #[error("channel config does not exist at path: {0}")]
    ChannelConfigDoesNotExist(String),

    #[error("no channels have been loaded; please review your lineup config")]
    NoChannelsLoaded,

    #[error("unable to find channel with number {0}")]
    ChannelNotFound(String),
}

impl IntoResponse for LineupError {
    fn into_response(self) -> Response {
        let status = match self {
            LineupError::ChannelNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}
