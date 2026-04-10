use std::fmt::Formatter;
use std::time::Duration;

use crate::error::FFPipelineError;
use crate::frame_rate::FrameRate;
use crate::input::InputSettings;
use crate::output::OutputSettings;
use crate::probe::ProbeResultStream;

const KEYFRAME_INTERVAL_SECONDS: u32 = 2;
pub const SEGMENT_SECONDS: u32 = 4;

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
    Cuda,
    VideoToolbox,
}

impl HardwareAccel {
    fn as_arg(&self) -> String {
        match self {
            HardwareAccel::Cuda => String::from("cuda"),
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
    StandardFormatFlags,
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
            GlobalOption::StandardFormatFlags => vec![
                String::from("-fflags"),
                String::from("+genpts+discardcorrupt+igndts"),
            ],
        }
    }
}

#[derive(Debug)]
pub enum OutputFormat {
    Hls {
        playlist: String,
        segment_template: String,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct PtsOffset {
    pub duration: Duration,
}

impl OutputFormat {
    fn as_arg(&self, output_context: &OutputContext) -> Vec<String> {
        let force_key_frames_expr = format!("expr:gte(t,n_forced*{KEYFRAME_INTERVAL_SECONDS})");
        let segment_seconds = format!("{SEGMENT_SECONDS}");
        let rounded_frame_rate = output_context
            .media_frame_rate
            .parsed_frame_rate
            .round_ties_even() as u32;

        // TODO: 1-second GOP for qsv
        let gop = format!("{}", rounded_frame_rate * KEYFRAME_INTERVAL_SECONDS);
        let keyint_min = format!("{}", rounded_frame_rate * KEYFRAME_INTERVAL_SECONDS);

        let mut args: Vec<&str> = Vec::new();

        match self {
            OutputFormat::Hls {
                segment_template, ..
            } => {
                match output_context.video_codec {
                    VideoCodec::Copy => {}
                    _ => {
                        args.extend(vec![
                            "-g",
                            &gop,
                            "-keyint_min",
                            &keyint_min,
                            "-force_key_frames",
                            &force_key_frames_expr,
                        ]);
                    }
                }

                args.extend(vec![
                    "-f",
                    "hls",
                    "-hls_time",
                    &segment_seconds,
                    "-hls_list_size",
                    "0",
                    "-segment_list_flags",
                    "+live",
                    "-hls_segment_filename",
                    segment_template,
                    "-hls_segment_type",
                    "mpegts",
                    "-hls_flags",
                    "program_date_time+omit_endlist+append_list+independent_segments",
                ]);

                match output_context.pts_offset {
                    Some(pts_offset) if pts_offset.duration > Duration::ZERO => {}
                    _ => args.extend(vec![
                        "-hls_segment_options",
                        "mpegts_flags=+initial_discontinuity",
                    ]),
                }
            }
        }

        args.into_iter().map(String::from).collect()
    }

    fn path(&self) -> String {
        match self {
            OutputFormat::Hls { playlist, .. } => playlist.clone(),
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
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

#[derive(Copy, Clone, PartialEq)]
pub enum VideoCodec {
    Copy,
    H264Nvenc,
    HevcNvenc,
    H264VideoToolbox,
    HevcVideoToolbox,
    Libx264,
    Libx265,
}

impl VideoCodec {
    fn as_arg(&self) -> Vec<String> {
        let codec = match self {
            VideoCodec::Copy => String::from("copy"),
            VideoCodec::H264Nvenc => String::from("h264_nvenc"),
            VideoCodec::HevcNvenc => String::from("hevc_nvenc"),
            VideoCodec::H264VideoToolbox => String::from("h264_videotoolbox"),
            VideoCodec::HevcVideoToolbox => String::from("hevc_videotoolbox"),
            VideoCodec::Libx264 => String::from("libx264"),
            VideoCodec::Libx265 => String::from("libx265"),
        };

        let mut result = vec![String::from("-vcodec"), codec];

        if self == &VideoCodec::Libx265 {
            result.extend([
                String::from("-tag:v"),
                String::from("hvc1"),
                String::from("-x265-params"),
                String::from("log-level=error"),
            ]);
        };

        result
    }
}

struct OutputContext {
    media_frame_rate: FrameRate,
    audio_codec: AudioCodec,
    video_codec: VideoCodec,
    pts_offset: Option<PtsOffset>,
}

pub enum OutputOption {
    Format(OutputFormat),
    VideoCodec(VideoCodec),
    VideoBitrate(Option<Kbps>),
    VideoBuffer(Option<Kbps>),
    AudioCodec(AudioCodec),
    AudioBitrate(Option<Kbps>),
    AudioBuffer(Option<Kbps>),
    Duration(Duration),
    TsOffset(Option<PtsOffset>),
    NoDemuxDecodeDelay,
    MovFlagsFastStart,
}

impl OutputOption {
    fn as_arg(&self, output_context: &OutputContext) -> Vec<String> {
        match self {
            OutputOption::Format(format) => format.as_arg(output_context),
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
                vec![String::from("-t"), format!("{}ms", duration.as_millis())]
            }
            OutputOption::TsOffset(Some(pts_offset)) if pts_offset.duration > Duration::ZERO => {
                vec![
                    String::from("-output_ts_offset"),
                    format!("{}ms", pts_offset.duration.as_millis()),
                ]
            }
            OutputOption::TsOffset(_) => Vec::new(),
            OutputOption::NoDemuxDecodeDelay => vec!["-muxdelay", "0", "-muxpreload", "0"]
                .into_iter()
                .map(String::from)
                .collect(),
            OutputOption::MovFlagsFastStart => {
                vec![String::from("-movflags"), String::from("+faststart")]
            }
        }
    }
}

pub enum PipelineInput {
    Video {
        path: String,
        seek: Duration,
        realtime: bool,
    },
}

pub struct PipelineOutput {
    path: String,
}

pub struct Pipeline {
    global_options: Vec<GlobalOption>,
    inputs: Vec<PipelineInput>,
    output_options: Vec<OutputOption>,
    output: PipelineOutput,

    output_context: OutputContext,
}

impl Pipeline {
    fn full(input_settings: InputSettings, output_settings: OutputSettings) -> Pipeline {
        let duration = input_settings.input.out_point - input_settings.input.in_point;

        let audio_codec = match output_settings.audio_format {
            Some(AudioFormat::Aac) => AudioCodec::Aac,
            Some(AudioFormat::Ac3) => AudioCodec::Ac3,
            _ => AudioCodec::Copy,
        };

        let video_codec = match (output_settings.accel, output_settings.video_format) {
            (Some(HardwareAccel::Cuda), Some(VideoFormat::H264)) => VideoCodec::H264Nvenc,
            (Some(HardwareAccel::Cuda), Some(VideoFormat::Hevc)) => VideoCodec::HevcNvenc,
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

        let output_path = output_settings.format.path();

        let media_frame_rate = input_settings
            .input
            .probe_result
            .streams
            .iter()
            .find_map(|s| match s {
                ProbeResultStream::Video(video_stream) => Some(video_stream.frame_rate.to_owned()),
                _ => None,
            })
            .unwrap_or(FrameRate::parse("24"));

        let output_context = OutputContext {
            audio_codec,
            video_codec,
            pts_offset: output_settings.pts_offset,
            media_frame_rate,
        };

        Pipeline {
            global_options: vec![
                GlobalOption::Threads(0),
                GlobalOption::NoStdIn,
                GlobalOption::HideBanner,
                GlobalOption::LogLevel(LogLevel::Error),
                GlobalOption::StandardFormatFlags,
                GlobalOption::HardwareAccel(output_settings.accel),
            ],
            inputs: vec![PipelineInput::Video {
                path: input_settings.input.probe_result.path,
                seek: input_settings.input.in_point,
                realtime: output_settings.realtime,
            }],
            output_options: vec![
                OutputOption::NoDemuxDecodeDelay,
                OutputOption::MovFlagsFastStart,
                OutputOption::AudioCodec(audio_codec),
                OutputOption::AudioBitrate(output_settings.audio_bitrate),
                OutputOption::AudioBuffer(output_settings.audio_buffer),
                OutputOption::VideoCodec(video_codec),
                OutputOption::VideoBitrate(output_settings.video_bitrate),
                OutputOption::VideoBuffer(output_settings.video_buffer),
                OutputOption::Format(output_settings.format),
                OutputOption::Duration(duration),
                OutputOption::TsOffset(output_settings.pts_offset),
            ],
            output: PipelineOutput { path: output_path },
            output_context,
        }
    }

    pub fn optimize(&mut self) {
        // audio copy shouldn't have bitrate etc
        if self.output_context.audio_codec == AudioCodec::Copy {
            self.output_options.retain(|o| {
                !matches!(
                    o,
                    OutputOption::AudioBitrate(_) | OutputOption::AudioBuffer(_)
                )
            });
        };

        // video copy shouldn't have bitrate, hwaccel, etc
        if self.output_context.video_codec == VideoCodec::Copy {
            self.global_options
                .retain(|o| !matches!(o, GlobalOption::HardwareAccel(_)));

            self.output_options.retain(|o| {
                !matches!(
                    o,
                    OutputOption::VideoBitrate(_) | OutputOption::VideoBuffer(_)
                )
            });
        }
    }

    pub fn args(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();

        result.extend(self.global_options.iter().flat_map(|o| o.as_arg()));

        for input in &self.inputs {
            match input {
                PipelineInput::Video {
                    path,
                    seek,
                    realtime,
                } => {
                    if !seek.is_zero() {
                        result.extend([String::from("-ss"), format!("{}ms", seek.as_millis())]);
                    }

                    if *realtime {
                        result.extend([String::from("-readrate"), String::from("1.0")]);
                    }

                    result.extend([String::from("-i"), path.to_owned()])
                }
            }
        }

        // TODO: filter_complex

        result.extend(
            self.output_options
                .iter()
                .flat_map(|o| o.as_arg(&self.output_context)),
        );

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
    input_settings: InputSettings,
    output_settings: OutputSettings,
) -> Result<Pipeline, FFPipelineError> {
    Ok(Pipeline::full(input_settings, output_settings))
}
