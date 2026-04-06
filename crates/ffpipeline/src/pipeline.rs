use std::fmt::Formatter;
use std::time::Duration;

use crate::error::FFPipelineError;
use crate::output::OutputSettings;
use crate::probe::ProbeResult;

#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Aac,
    Ac3,
}

#[derive(Debug, Clone, Copy)]
pub struct Kbps(pub u32);

#[derive(Debug, Clone, Copy)]
pub enum VideoFormat {
    H264,
    Hevc,
}

#[derive(Debug, Clone, Copy)]
pub enum HardwareAccel {
    VideoToolbox,
}

impl HardwareAccel {
    fn as_arg(&self) -> String {
        match self {
            HardwareAccel::VideoToolbox => String::from("videotoolbox"),
        }
    }
}

pub enum LogLevel {
    Error,
}

impl LogLevel {
    fn as_arg(&self) -> String {
        match self {
            LogLevel::Error => String::from("error"),
        }
    }
}

pub enum GlobalOption {
    Threads(u32),
    NoStdIn,
    HideBanner,
    LogLevel(LogLevel),
    HardwareAccel(Option<HardwareAccel>),
}

impl GlobalOption {
    fn as_arg(&self) -> Vec<String> {
        match self {
            GlobalOption::Threads(count) => vec![String::from("-threads"), count.to_string()],
            GlobalOption::NoStdIn => vec![String::from("-nostdin")],
            GlobalOption::HideBanner => vec![String::from("-hide_banner")],
            GlobalOption::LogLevel(level) => vec![String::from("-loglevel"), level.as_arg()],
            GlobalOption::HardwareAccel(Some(hardware_accel)) => {
                vec![String::from("-hwaccel"), hardware_accel.as_arg()]
            }
            GlobalOption::HardwareAccel(None) => Vec::new(),
        }
    }
}

pub enum OutputFormat {
    Hls,
}

impl OutputFormat {
    fn as_arg(&self) -> Vec<String> {
        match self {
            OutputFormat::Hls => [
                "-f",
                "hls",
                "-hls_time",
                "4",
                "-hls_list_size",
                "0",
                "-segment_list_flags",
                "+live",
                "-hls_segment_type",
                "mpegts",
            ]
            .map(String::from)
            .to_vec(),
        }
    }
}

pub enum AudioCodec {
    Copy,
    Aac,
    Ac3,
}

impl AudioCodec {
    fn as_arg(&self) -> Vec<String> {
        let codec = match self {
            AudioCodec::Copy => String::from("copy"),
            AudioCodec::Aac => String::from("aac"),
            AudioCodec::Ac3 => String::from("ac3"),
        };

        vec![String::from("-acodec"), codec]
    }
}

pub enum VideoCodec {
    Copy,
    H264VideoToolbox,
    HevcVideoToolbox,
    Libx264,
    Libx265,
}

impl VideoCodec {
    fn as_arg(&self) -> Vec<String> {
        let codec = match self {
            VideoCodec::Copy => String::from("copy"),
            VideoCodec::H264VideoToolbox => String::from("h264_videotoolbox"),
            VideoCodec::HevcVideoToolbox => String::from("hevc_videotoolbox"),
            VideoCodec::Libx264 => String::from("libx264"),
            VideoCodec::Libx265 => String::from("libx265"),
        };

        vec![String::from("-vcodec"), codec]
    }
}

pub enum OutputOption {
    Format(OutputFormat),
    VideoCodec(VideoCodec),
    VideoBitrate(Option<Kbps>),
    VideoBuffer(Option<Kbps>),
    AudioCodec(AudioCodec),
    AudioBitrate(Option<Kbps>),
    AudioBuffer(Option<Kbps>),
    Duration(std::time::Duration),
}

impl OutputOption {
    fn as_arg(&self) -> Vec<String> {
        match self {
            OutputOption::Format(format) => format.as_arg(),
            OutputOption::VideoCodec(codec) => codec.as_arg(),
            OutputOption::VideoBitrate(Some(bitrate_kbps)) => {
                vec![
                    String::from("-b:v"),
                    format!("{}k", bitrate_kbps.0),
                    String::from("-maxrate:v"),
                    format!("{}k", bitrate_kbps.0),
                ]
            }
            OutputOption::VideoBitrate(None) => Vec::new(),
            OutputOption::VideoBuffer(Some(buffer_kbps)) => {
                vec![String::from("-bufsize:v"), format!("{}k", buffer_kbps.0)]
            }
            OutputOption::VideoBuffer(None) => Vec::new(),
            OutputOption::AudioCodec(codec) => codec.as_arg(),
            OutputOption::AudioBitrate(Some(bitrate_kbps)) => {
                vec![
                    String::from("-b:a"),
                    format!("{}k", bitrate_kbps.0),
                    String::from("-maxrate:a"),
                    format!("{}k", bitrate_kbps.0),
                ]
            }
            OutputOption::AudioBitrate(None) => Vec::new(),
            OutputOption::AudioBuffer(Some(buffer_kbps)) => {
                vec![String::from("-bufsize:a"), format!("{}k", buffer_kbps.0)]
            }
            OutputOption::AudioBuffer(None) => Vec::new(),
            OutputOption::Duration(duration) => {
                vec![String::from("-t"), format!("{}s", duration.as_secs_f64())]
            }
        }
    }
}

pub enum PipelineInput {
    Video { path: String, seek: Duration },
}

pub struct PipelineOutput {
    path: String,
}

pub struct Pipeline {
    global_options: Vec<GlobalOption>,
    inputs: Vec<PipelineInput>,
    output_options: Vec<OutputOption>,
    output: PipelineOutput,
}

impl Pipeline {
    fn full(
        seek: Duration,
        probe_result: ProbeResult,
        output_settings: OutputSettings,
        output: String,
    ) -> Pipeline {
        // for now, limit to 30s
        let duration = match probe_result.duration {
            Some(probed_duration) => probed_duration.min(std::time::Duration::from_secs(30)),
            None => std::time::Duration::from_secs(30),
        };

        let audio_codec = match output_settings.audio_format {
            Some(AudioFormat::Aac) => AudioCodec::Aac,
            Some(AudioFormat::Ac3) => AudioCodec::Ac3,
            _ => AudioCodec::Copy,
        };

        let video_codec = match (output_settings.accel, output_settings.video_format) {
            (Some(HardwareAccel::VideoToolbox), Some(VideoFormat::H264)) => {
                VideoCodec::H264VideoToolbox
            }
            (Some(HardwareAccel::VideoToolbox), Some(VideoFormat::Hevc)) => {
                VideoCodec::HevcVideoToolbox
            }
            (None, Some(VideoFormat::H264)) => VideoCodec::Libx264,
            (None, Some(VideoFormat::Hevc)) => VideoCodec::Libx265,
            _ => VideoCodec::Copy,
        };

        Pipeline {
            global_options: vec![
                GlobalOption::Threads(0),
                GlobalOption::NoStdIn,
                GlobalOption::HideBanner,
                GlobalOption::LogLevel(LogLevel::Error),
                GlobalOption::HardwareAccel(output_settings.accel),
            ],
            inputs: vec![PipelineInput::Video {
                path: probe_result.path,
                seek,
            }],
            output_options: vec![
                OutputOption::AudioCodec(audio_codec),
                OutputOption::AudioBitrate(output_settings.audio_bitrate),
                OutputOption::AudioBuffer(output_settings.audio_buffer),
                OutputOption::VideoCodec(video_codec),
                OutputOption::VideoBitrate(output_settings.video_bitrate),
                OutputOption::VideoBuffer(output_settings.video_buffer),
                OutputOption::Format(OutputFormat::Hls),
                OutputOption::Duration(duration),
            ],
            output: PipelineOutput { path: output },
        }
    }

    pub fn args(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();

        result.extend(self.global_options.iter().flat_map(|o| o.as_arg()));

        for input in &self.inputs {
            match input {
                PipelineInput::Video { path, seek } => {
                    if !seek.is_zero() {
                        result.extend([String::from("-ss"), format!("{}ms", seek.as_millis())]);
                    }

                    result.extend([String::from("-i"), path.to_owned()])
                }
            }
        }

        // TODO: filter_complex

        result.extend(self.output_options.iter().flat_map(|o| o.as_arg()));

        result.extend([self.output.path.to_owned()]);

        result
    }
}

impl std::fmt::Display for Pipeline {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "args: {}", self.args().join(" "))
    }
}

pub fn generate_pipeline(
    seek: Duration,
    probe_result: ProbeResult,
    output_settings: OutputSettings,
    output: String,
) -> Result<Pipeline, FFPipelineError> {
    Ok(Pipeline::full(seek, probe_result, output_settings, output))
}
