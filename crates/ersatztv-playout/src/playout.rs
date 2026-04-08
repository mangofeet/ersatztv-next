use std::path::Path;

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
    #[serde(with = "time::serde::rfc3339")]
    pub finish: OffsetDateTime,
    pub in_point_ms: Option<u64>,
    pub out_point_ms: Option<u64>,
    pub source: Option<PlayoutItemSource>,
    pub tracks: Option<PlayoutItemTracks>,
}

impl PlayoutItem {
    pub fn new(
        id: String,
        start: OffsetDateTime,
        finish: OffsetDateTime,
        in_point: Option<std::time::Duration>,
        out_point: Option<std::time::Duration>,
        path: &Path,
    ) -> Result<PlayoutItem, PlayoutError> {
        Ok(PlayoutItem {
            id,
            start,
            finish,
            in_point_ms: in_point.map(|d| d.as_millis() as u64),
            out_point_ms: out_point.map(|d| d.as_millis() as u64),
            source: Some(PlayoutItemSource::Local {
                path: path.to_string_lossy().to_string(),
            }),
            tracks: None,
        })
    }

    pub fn finish(&self) -> OffsetDateTime {
        self.finish
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
