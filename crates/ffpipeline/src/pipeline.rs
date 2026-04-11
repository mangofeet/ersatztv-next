use std::collections::HashSet;
use std::fmt::Formatter;
use std::time::Duration;

use crate::audio_codec::AudioCodec;
use crate::error::FFPipelineError;
use crate::frame_rate::FrameRate;
use crate::global_option::{GlobalOption, LogLevel};
use crate::hardware_accel::HardwareAccel;
use crate::input::InputSettings;
use crate::output_option::OutputOption;
use crate::output_settings::OutputSettings;
use crate::probe::{ProbeResultAudioStream, ProbeResultStream, ProbeResultVideoStream};
use crate::video_codec::VideoCodec;

pub const KEYFRAME_INTERVAL_SECONDS: u32 = 2;
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

#[derive(Debug, Copy, Clone)]
pub struct PtsOffset {
    pub duration: Duration,
}

pub(crate) struct OutputContext {
    pub(crate) media_frame_rate: FrameRate,
    pub(crate) audio_codec: AudioCodec,
    pub(crate) audio_channels: Option<u32>,
    pub(crate) video_codec: VideoCodec,
    pub(crate) pts_offset: Option<PtsOffset>,
}

pub enum PipelineInput {
    Audio {
        index: Option<u32>,
        path: String,
        channels: u32,
    },
    Video {
        index: Option<u32>,
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
            (Some(HardwareAccel::Qsv), Some(VideoFormat::H264)) => VideoCodec::H264Qsv,
            (Some(HardwareAccel::Qsv), Some(VideoFormat::Hevc)) => VideoCodec::HevcQsv,
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

        let video_stream = Self::select_video_stream(&input_settings);
        let audio_stream = Self::select_audio_stream(&input_settings);

        let media_frame_rate = video_stream
            .map(|v| v.frame_rate.to_owned())
            .unwrap_or(FrameRate::parse("24"));

        // should we fail instead of assuming 2 audio channels?
        let audio_channels = audio_stream.map(|a| a.channels).unwrap_or(2);

        let output_context = OutputContext {
            audio_codec,
            audio_channels: output_settings.audio_channels,
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
            inputs: vec![
                PipelineInput::Audio {
                    index: audio_stream.map(|a| a.stream_index),
                    path: input_settings.input.probe_result.path.to_owned(),
                    channels: audio_channels,
                },
                PipelineInput::Video {
                    index: video_stream.map(|v| v.stream_index),
                    path: input_settings.input.probe_result.path.to_owned(),
                    seek: input_settings.input.in_point,
                    realtime: output_settings.realtime,
                },
            ],
            output_options: vec![
                OutputOption::NoDemuxDecodeDelay,
                OutputOption::MovFlagsFastStart,
                OutputOption::AudioCodec(audio_codec),
                OutputOption::AudioBitrate(output_settings.audio_bitrate),
                OutputOption::AudioBuffer(output_settings.audio_buffer),
                OutputOption::AudioChannels(output_settings.audio_channels),
                OutputOption::VideoCodec(video_codec),
                OutputOption::VideoBitrate(output_settings.video_bitrate),
                OutputOption::VideoBuffer(output_settings.video_buffer),
                OutputOption::DoNotMapMetadata,
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
                    OutputOption::AudioBitrate(_)
                        | OutputOption::AudioBuffer(_)
                        | OutputOption::AudioChannels(_)
                )
            });
        };

        // remove audio channels output option if input channel count matches
        if let Some(audio_channels) = self.inputs.iter().find_map(|s| match s {
            PipelineInput::Audio { channels, .. } => Some(channels),
            _ => None,
        }) && Some(audio_channels) == self.output_context.audio_channels.as_ref()
        {
            self.output_options
                .retain(|o| !matches!(o, OutputOption::AudioChannels(_)));
        }

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

        let mut audio_label = String::from("0:a");
        let mut video_label = String::from("0:v");

        let mut distinct_paths: HashSet<&str> = HashSet::new();
        for input in &self.inputs {
            match input {
                PipelineInput::Audio { path, .. } => {
                    distinct_paths.insert(path);
                }
                PipelineInput::Video { path, .. } => {
                    distinct_paths.insert(path);
                }
            }
        }

        result.extend(self.global_options.iter().flat_map(|o| o.as_arg()));

        for input in &self.inputs {
            match input {
                // TODO: need to check if audio path differs from video path
                PipelineInput::Audio { index, path, .. } => {
                    let audio_input_index =
                        distinct_paths.iter().position(|p| p == path).unwrap_or(0);
                    if let Some(index) = index {
                        audio_label = format!("{}:{}", audio_input_index, index);
                    }
                }
                PipelineInput::Video {
                    index,
                    path,
                    seek,
                    realtime,
                    ..
                } => {
                    let video_input_index =
                        distinct_paths.iter().position(|p| p == path).unwrap_or(0);
                    if let Some(index) = index {
                        video_label = format!("{}:{}", video_input_index, index);
                    }

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

        result.extend([String::from("-map"), video_label]);
        result.extend([String::from("-map"), audio_label]);

        result.extend(
            self.output_options
                .iter()
                .flat_map(|o| o.as_arg(&self.output_context)),
        );

        result.extend([self.output.path.to_owned()]);

        result
    }

    fn select_video_stream(input_settings: &InputSettings) -> Option<&ProbeResultVideoStream> {
        let mut all_video_streams: Vec<&ProbeResultVideoStream> = input_settings
            .input
            .probe_result
            .streams
            .iter()
            .filter_map(|s| match s {
                ProbeResultStream::Video(video_stream) => Some(video_stream),
                _ => None,
            })
            .collect();

        if let Some(video_index) = input_settings.input.video_index {
            let matched_stream = all_video_streams
                .iter()
                .find(|v| v.stream_index == video_index);

            match matched_stream {
                Some(video_stream) => {
                    return Some(video_stream);
                }
                None => {
                    log::warn!(
                        "unable to locate requested video stream with index {}",
                        video_index
                    );
                }
            }
        }

        match all_video_streams.len() {
            0 => None,
            1 => Some(all_video_streams[0]),
            _ => {
                log::warn!(
                    "content contains more than one video stream; selecting stream with lowest index"
                );
                all_video_streams.sort_by_key(|v| v.stream_index);
                Some(all_video_streams[0])
            }
        }
    }

    fn select_audio_stream(input_settings: &InputSettings) -> Option<&ProbeResultAudioStream> {
        let mut all_audio_streams: Vec<&ProbeResultAudioStream> = input_settings
            .input
            .probe_result
            .streams
            .iter()
            .filter_map(|s| match s {
                ProbeResultStream::Audio(audio_stream) => Some(audio_stream),
                _ => None,
            })
            .collect();

        if let Some(audio_index) = input_settings.input.audio_index {
            let matched_stream = all_audio_streams
                .iter()
                .find(|a| a.stream_index == audio_index);

            match matched_stream {
                Some(audio_stream) => {
                    return Some(audio_stream);
                }
                None => {
                    log::warn!(
                        "unable to locate requested audio stream with index {}",
                        audio_index
                    );
                }
            }
        }

        match all_audio_streams.len() {
            0 => None,
            1 => Some(all_audio_streams[0]),
            _ => {
                log::warn!(
                    "content contains more than one audio stream; selecting stream with greatest number of channels"
                );
                all_audio_streams.sort_by_key(|a| std::cmp::Reverse(a.channels));
                Some(all_audio_streams[0])
            }
        }
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
