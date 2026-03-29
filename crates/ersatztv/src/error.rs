use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LineupError {
    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("lineup config is required as arg")]
    LineupConfigRequired,

    #[error("unable to load lineup config {0}")]
    LineupConfigFailure(String),

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
