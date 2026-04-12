use tokio::process::Command;

use crate::error::FFPipelineError;
use crate::pipeline::HardwareAccel;

static KNOWN_ACCELS: &[&str] = &["cuda", "qsv", "videotoolbox"];

#[derive(Debug)]
pub struct FfmpegInfo {
    hwaccels: Vec<String>,
}

impl FfmpegInfo {
    pub async fn load(path: &str) -> Result<FfmpegInfo, FFPipelineError> {
        let hwaccels = Self::load_hw_accels(path).await?;
        //let video_filters = Self::load_video_filters(path).await?;
        Ok(FfmpegInfo { hwaccels })
    }

    pub fn has_hw_accel(&self, accel: &HardwareAccel) -> bool {
        let accel_string = match accel {
            HardwareAccel::Cuda => String::from("cuda"),
            HardwareAccel::Qsv => String::from("qsv"),
            HardwareAccel::VideoToolbox => String::from("videotoolbox"),
        };

        self.hwaccels.contains(&accel_string)
    }

    async fn load_hw_accels(path: &str) -> Result<Vec<String>, FFPipelineError> {
        let mut hwaccels: Vec<String> = Vec::new();

        let output = Command::new(path)
            .args(["-v", "quiet", "-hwaccels"])
            .output()
            .await
            .map_err(|_| FFPipelineError::FfmpegCapabilitiesError(String::from("hwaccels")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();

            if trimmed.contains(":") || trimmed.is_empty() {
                continue;
            }

            if KNOWN_ACCELS.contains(&trimmed) {
                hwaccels.push(trimmed.to_owned());
            }
        }

        Ok(hwaccels)
    }
}
