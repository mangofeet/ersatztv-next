use std::path::Path;

use tokio::process::Command;

use crate::error::FFPipelineError;
use crate::hw_accel::{HardwareAccel, HwAccel};

static KNOWN_ACCELS: &[&str] = &["cuda", "qsv", "vaapi", "videotoolbox"];
static KNOWN_FILTERS: &[&str] = &[
    "pad_cuda",
    "pad_vaapi",
    "scale_cuda",
    "scale_vaapi",
    "vpp_qsv",
];

pub enum KnownVideoFilter {
    PadCuda,
    PadVaapi,
    ScaleCuda,
    ScaleVaapi,
    VppQsv,
}

#[derive(Debug, Clone, Default)]
pub struct FfmpegInfo {
    hwaccels: Vec<String>,
    video_filters: Vec<String>,
}

impl FfmpegInfo {
    pub async fn load(
        path: &Path,
        disabled_filters: &[String],
    ) -> Result<FfmpegInfo, FFPipelineError> {
        let hwaccels = Self::load_hw_accels(path).await?;
        let video_filters = Self::load_video_filters(path, disabled_filters).await?;
        Ok(FfmpegInfo {
            hwaccels,
            video_filters,
        })
    }

    pub fn has_hw_accel(&self, accel: &HardwareAccel) -> bool {
        if let Some(accel_string) = Some(accel.ffmpeg_name()) {
            self.hwaccels.iter().any(|f| f == accel_string)
        } else {
            false
        }
    }

    pub fn has_video_filter(&self, filter: &KnownVideoFilter) -> bool {
        if let Some(filter_string) = match filter {
            KnownVideoFilter::PadCuda => Some("pad_cuda"),
            KnownVideoFilter::PadVaapi => Some("pad_vaapi"),
            KnownVideoFilter::ScaleCuda => Some("scale_cuda"),
            KnownVideoFilter::ScaleVaapi => Some("scale_vaapi"),
            KnownVideoFilter::VppQsv => Some("vpp_qsv"),
        } {
            self.video_filters.iter().any(|f| f == filter_string)
        } else {
            false
        }
    }

    async fn load_hw_accels(path: &Path) -> Result<Vec<String>, FFPipelineError> {
        let output = Command::new(path)
            .args(["-hide_banner", "-hwaccels"])
            .output()
            .await
            .map_err(|_| FFPipelineError::FfmpegCapabilitiesError(String::from("hwaccels")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut accels: Vec<String> = Vec::new();

        for line in stdout.lines() {
            let trimmed = line.trim();

            if trimmed.contains(":") || trimmed.is_empty() {
                continue;
            }

            if KNOWN_ACCELS.contains(&trimmed) {
                accels.push(trimmed.to_owned());
            }
        }

        Ok(accels)
    }

    async fn load_video_filters(
        path: &Path,
        disabled_filters: &[String],
    ) -> Result<Vec<String>, FFPipelineError> {
        let output = Command::new(path)
            .args(["-hide_banner", "-filters"])
            .output()
            .await
            .map_err(|_| FFPipelineError::FfmpegCapabilitiesError(String::from("filters")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut filters: Vec<String> = Vec::new();

        for line in stdout.lines() {
            //  .. scale_cuda        V->V       GPU accelerated video resizer
            if let Some(filter) = line.split_whitespace().nth(1)
                && KNOWN_FILTERS.contains(&filter)
                && !disabled_filters.iter().any(|f| f == filter)
            {
                filters.push(filter.to_owned());
            }
        }

        Ok(filters)
    }
}
