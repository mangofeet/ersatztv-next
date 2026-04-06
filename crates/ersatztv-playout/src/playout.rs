use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::PlayoutError;

#[derive(Debug, Deserialize, Serialize)]
pub struct Playout {
    pub version: String,
    pub items: Vec<PlayoutItem>,
}

impl Playout {
    pub fn new(items: Vec<PlayoutItem>) -> Self {
        Playout {
            version: String::from("https://ersatztv.org/playout/version/0.0.1"),
            items,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayoutItem {
    pub id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    pub source: Option<PlayoutItemSource>,
    pub tracks: Option<PlayoutItemTracks>,
    pub duration_ms: u64,
}

impl PlayoutItem {
    pub fn new(
        id: String,
        start: OffsetDateTime,
        path: &Path,
        duration: Duration,
    ) -> Result<PlayoutItem, PlayoutError> {
        Ok(PlayoutItem {
            id,
            start,
            source: Some(PlayoutItemSource::Local {
                path: path.to_string_lossy().to_string(),
            }),
            tracks: None,
            duration_ms: duration.as_millis() as u64,
        })
    }

    pub fn finish(&self) -> OffsetDateTime {
        self.start + Duration::from_millis(self.duration_ms)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayoutItemTracks {
    pub audio: Option<PlayoutItemAudioTrack>,
    pub video: Option<PlayoutItemVideoTrack>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayoutItemAudioTrack {
    pub source: Option<PlayoutItemSource>,
    pub stream_index: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayoutItemVideoTrack {
    pub source: PlayoutItemSource,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum PlayoutItemSource {
    Local { path: String },
    Lavfi { params: String },
}

pub struct PlayoutLoadResult {
    pub playout: Playout,
    // TODO: start, finish
}

pub async fn from_file(path: &str) -> Result<PlayoutLoadResult, PlayoutError> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| PlayoutError::PlayoutJsonLoadError(e.to_string()))?;

    let playout: Playout = serde_json::from_str(&contents)
        .map_err(|e| PlayoutError::PlayoutJsonLoadError(e.to_string()))?;

    Ok(PlayoutLoadResult { playout })
}
