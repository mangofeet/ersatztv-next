use std::fmt::Formatter;
use std::time::Duration;

use crate::audio_codec::AudioCodec;
use crate::audio_decoder::AudioDecoder;
use crate::audio_filter::AudioFilter;
use crate::error::FFPipelineError;
use crate::filter_chain::{FilterChain, PipelineFilter};
use crate::frame_rate::FrameRate;
use crate::frame_size::FrameSize;
use crate::global_option::{GlobalOption, LogLevel};
use crate::input::{InputSettings, InputSource};
use crate::output_option::OutputOption;
use crate::output_settings::OutputSettings;
use crate::probe::{ProbeResultAudioStream, ProbeResultStream, ProbeResultVideoStream};
use crate::video_codec::VideoCodec;
use crate::video_decoder::VideoDecoder;
use crate::video_filter::VideoFilter;

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

#[derive(Debug, Clone, Copy)]
pub enum HardwareAccel {
    Cuda,
    Qsv,
    VideoToolbox,
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
    pub(crate) preferred_surface: FrameSurface,
    pub(crate) preferred_pixel_format: Option<PixelFormat>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FrameSurface {
    System,
    Cuda,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PixelFormat {
    Yuv420p,
    Yuv420p10le,
    Nv12,
    P010le,
}

impl PixelFormat {
    pub(crate) fn parse(pix_fmt: &str) -> PixelFormat {
        match pix_fmt {
            "yuv420p" => PixelFormat::Yuv420p,
            "yuv420p10le" => PixelFormat::Yuv420p10le,
            "nv12" => PixelFormat::Nv12,
            "p010le" => PixelFormat::P010le,
            _ => {
                log::warn!("assuming unknown pixel format {} is yuv420p", pix_fmt);
                PixelFormat::Yuv420p
            }
        }
    }

    pub(crate) fn bit_depth(&self) -> u32 {
        match self {
            PixelFormat::Yuv420p | PixelFormat::Nv12 => 8,
            PixelFormat::Yuv420p10le | PixelFormat::P010le => 10,
        }
    }

    pub(crate) fn as_arg(&self) -> &str {
        match self {
            PixelFormat::Yuv420p => "yuv420p",
            PixelFormat::Yuv420p10le => "yuv420p10le",
            PixelFormat::Nv12 => "nv12",
            PixelFormat::P010le => "p010le",
        }
    }
}

#[derive(Clone)]
pub(crate) struct FrameState {
    pub(crate) size: FrameSize,
    pub(crate) is_anamorphic: bool,
    pub(crate) sample_aspect_ratio: Option<String>,
    pub(crate) display_aspect_ratio: Option<String>,
    pub(crate) surface: FrameSurface,
    pub(crate) pixel_format: PixelFormat,
}

pub enum PipelineInput {
    Audio {
        input_source: InputSource,
        index: u32,
        path: String,
        seek: Duration,
        channels: u32,
        decoder: AudioDecoder,
    },
    Video {
        input_source: InputSource,
        index: u32,
        path: String,
        seek: Duration,
        realtime: bool,
        decoder: VideoDecoder,
    },
}

pub struct PipelineOutput {
    path: String,
}

pub struct Pipeline {
    accel: Option<HardwareAccel>,
    initial_state: FrameState,

    global_options: Vec<GlobalOption>,
    inputs: Vec<PipelineInput>,
    filter_chain: FilterChain,
    output_options: Vec<OutputOption>,
    output: PipelineOutput,

    output_context: OutputContext,
}

impl Pipeline {
    fn full(
        input_settings: InputSettings,
        output_settings: OutputSettings,
    ) -> Result<Pipeline, FFPipelineError> {
        let duration = std::cmp::min(
            input_settings.audio_input.out_point - input_settings.audio_input.in_point,
            input_settings.video_input.out_point - input_settings.video_input.in_point,
        );

        log::debug!("duration is {:?}", duration);

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

        let video_stream = Self::select_video_stream(&input_settings)?;
        let audio_stream = Self::select_audio_stream(&input_settings)?;

        let is_still_image = input_settings.video_input.probe_result.is_still_image();
        let video_decoder = VideoDecoder::new(video_stream, is_still_image, &output_settings);

        let initial_state = FrameState {
            size: FrameSize {
                width: video_stream.width,
                height: video_stream.height,
            },
            is_anamorphic: Self::is_anamorphic(video_stream),
            sample_aspect_ratio: video_stream.sample_aspect_ratio.to_owned(),
            display_aspect_ratio: video_stream.display_aspect_ratio.to_owned(),
            surface: video_decoder.output_surface(),
            pixel_format: PixelFormat::parse(video_stream.pix_fmt.as_str()),
        };

        let initial_scaled_size = output_settings
            .video_size
            .as_ref()
            .map(|s| s.square_pixel_size(&initial_state));

        let output_context = OutputContext {
            audio_codec,
            audio_channels: output_settings.audio_channels,
            video_codec,
            pts_offset: output_settings.pts_offset,
            media_frame_rate: video_stream.frame_rate.to_owned(),
            preferred_surface: match output_settings.accel {
                Some(HardwareAccel::Cuda) => FrameSurface::Cuda,
                // TODO: proper surfaces for other accels
                _ => FrameSurface::System,
            },
            preferred_pixel_format: video_codec
                .preferred_pixel_format(initial_state.pixel_format.bit_depth()),
        };

        Ok(Pipeline {
            accel: output_settings.accel,
            initial_state: initial_state.clone(),
            global_options: vec![
                GlobalOption::Threads(0),
                GlobalOption::NoStdIn,
                GlobalOption::HideBanner,
                GlobalOption::LogLevel(LogLevel::Error),
                GlobalOption::StandardFormatFlags,
            ],
            inputs: vec![
                PipelineInput::Audio {
                    input_source: input_settings.audio_input.input_source.to_owned(),
                    index: audio_stream.stream_index,
                    path: input_settings.audio_input.probe_result.path.to_owned(),
                    seek: input_settings.audio_input.in_point,
                    channels: audio_stream.channels,
                    decoder: AudioDecoder::new(audio_stream, &output_settings),
                },
                PipelineInput::Video {
                    input_source: input_settings.video_input.input_source.to_owned(),
                    index: video_stream.stream_index,
                    path: input_settings.video_input.probe_result.path.to_owned(),
                    seek: input_settings.video_input.in_point,
                    realtime: output_settings.realtime,
                    decoder: video_decoder,
                },
            ],
            filter_chain: FilterChain::new(vec![
                PipelineFilter::Audio(AudioFilter::Resample),
                PipelineFilter::Audio(AudioFilter::Pad),
                PipelineFilter::Video(VideoFilter::Loop {
                    codec: video_stream.codec.to_owned(),
                }),
                PipelineFilter::Video(VideoFilter::Scale {
                    size: initial_scaled_size,
                    input_is_anamorphic: initial_state.is_anamorphic,
                    force_original_aspect_ratio: None,
                }),
                PipelineFilter::Video(VideoFilter::Pad {
                    size: output_settings.video_size.to_owned(),
                }),
            ]),
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
                OutputOption::FrameRate(output_settings.frame_rate),
            ],
            output: PipelineOutput { path: output_path },
            output_context,
        })
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

            self.filter_chain.disable_audio();
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

        // video copy shouldn't have bitrate, etc
        if self.output_context.video_codec == VideoCodec::Copy {
            self.output_options.retain(|o| {
                !matches!(
                    o,
                    OutputOption::VideoBitrate(_) | OutputOption::VideoBuffer(_)
                )
            });

            self.filter_chain.disable_video();
        }

        self.filter_chain.evaluate(&self.initial_state);
        self.filter_chain.resolve(
            self.accel,
            &self.initial_state,
            &self.output_context.preferred_surface,
            &self.output_context.preferred_pixel_format,
        );

        if let Some(accel) = self.needs_hw_device() {
            self.global_options.push(GlobalOption::InitHwDevice(accel));
        }
    }

    pub fn args(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();

        let mut audio_label = String::from("0:a");
        let mut video_label = String::from("0:v");

        let mut distinct_paths: Vec<&str> = Vec::new();
        for input in &self.inputs {
            let path = match input {
                PipelineInput::Audio { path, .. } => path.as_str(),
                PipelineInput::Video { path, .. } => path.as_str(),
            };
            if !distinct_paths.contains(&path) {
                distinct_paths.push(path);
            }
        }

        result.extend(self.global_options.iter().flat_map(|o| o.as_arg()));

        for input in &self.inputs {
            match input {
                PipelineInput::Audio {
                    input_source,
                    index,
                    path,
                    decoder,
                    ..
                } => {
                    result.extend(decoder.as_arg());

                    let audio_input_index =
                        distinct_paths.iter().position(|p| p == path).unwrap_or(0);
                    audio_label = format!("{}:{}", audio_input_index, index);

                    // if more than one path, audio is probably separate from video
                    if distinct_paths.len() > 1 {
                        if matches!(input_source, InputSource::Lavfi { .. }) {
                            result.extend([String::from("-f"), String::from("lavfi")]);
                        }

                        result.extend([String::from("-i"), path.to_owned()]);
                    }
                }
                PipelineInput::Video {
                    input_source,
                    index,
                    path,
                    seek,
                    realtime,
                    decoder,
                    ..
                } => {
                    result.extend(decoder.as_arg());

                    let video_input_index =
                        distinct_paths.iter().position(|p| p == path).unwrap_or(0);
                    video_label = format!("{}:{}", video_input_index, index);

                    if !seek.is_zero() {
                        result.extend([String::from("-ss"), format!("{}ms", seek.as_millis())]);
                    }

                    if *realtime {
                        result.extend([String::from("-readrate"), String::from("1.0")]);
                    }

                    if matches!(input_source, InputSource::Lavfi { .. }) {
                        result.extend([String::from("-f"), String::from("lavfi")]);
                    }

                    result.extend([String::from("-i"), path.to_owned()]);
                }
            }
        }

        let mut filter_chain = self.filter_chain.to_owned();
        filter_chain.build(&audio_label, &video_label);

        result.extend(filter_chain.as_arg());

        result.extend([String::from("-map"), filter_chain.video_label().to_owned()]);
        result.extend([String::from("-map"), filter_chain.audio_label().to_owned()]);

        result.extend(
            self.output_options
                .iter()
                .flat_map(|o| o.as_arg(&self.output_context)),
        );

        result.extend([self.output.path.to_owned()]);

        result
    }

    fn select_video_stream(
        input_settings: &InputSettings,
    ) -> Result<&ProbeResultVideoStream, FFPipelineError> {
        let mut all_video_streams: Vec<&ProbeResultVideoStream> = input_settings
            .video_input
            .probe_result
            .streams
            .iter()
            .filter_map(|s| match s {
                ProbeResultStream::Video(video_stream) => Some(video_stream),
                _ => None,
            })
            .collect();

        if let Some(video_index) = input_settings.video_input.video_index {
            let matched_stream = all_video_streams
                .iter()
                .find(|v| v.stream_index == video_index);

            match matched_stream {
                Some(video_stream) => {
                    return Ok(video_stream);
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
            0 => Err(FFPipelineError::VideoInputIsRequired),
            1 => Ok(all_video_streams[0]),
            _ => {
                log::warn!(
                    "content contains more than one video stream; selecting stream with lowest index"
                );
                all_video_streams.sort_by_key(|v| v.stream_index);
                Ok(all_video_streams[0])
            }
        }
    }

    fn select_audio_stream(
        input_settings: &InputSettings,
    ) -> Result<&ProbeResultAudioStream, FFPipelineError> {
        let mut all_audio_streams: Vec<&ProbeResultAudioStream> = input_settings
            .audio_input
            .probe_result
            .streams
            .iter()
            .filter_map(|s| match s {
                ProbeResultStream::Audio(audio_stream) => Some(audio_stream),
                _ => None,
            })
            .collect();

        if let Some(audio_index) = input_settings.audio_input.audio_index {
            let matched_stream = all_audio_streams
                .iter()
                .find(|a| a.stream_index == audio_index);

            match matched_stream {
                Some(audio_stream) => {
                    return Ok(audio_stream);
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
            0 => Err(FFPipelineError::AudioInputIsRequired),
            1 => Ok(all_audio_streams[0]),
            _ => {
                log::warn!(
                    "content contains more than one audio stream; selecting stream with greatest number of channels"
                );
                all_audio_streams.sort_by_key(|a| std::cmp::Reverse(a.channels));
                Ok(all_audio_streams[0])
            }
        }
    }

    fn is_anamorphic(video_stream: &ProbeResultVideoStream) -> bool {
        // TODO: need to calculate SAR when it's not provided; port MediaStream::SampleAspectRatio

        match &video_stream.sample_aspect_ratio {
            Some(sample_aspect_ratio) => {
                let display_aspect_ratio = video_stream
                    .display_aspect_ratio
                    .as_ref()
                    .map(|dar| dar.as_ref())
                    .unwrap_or("");

                // square pixels
                if sample_aspect_ratio == "1:1" {
                    false
                }
                // 0:1 is "unspecified", so anything other than that will be non-square/anamorphic
                else if sample_aspect_ratio != "0:1" {
                    true
                }
                // SAR 0:1 && DAR 0:1 (both unspecified) means square pixels
                else if display_aspect_ratio == "0:1" {
                    false
                } else {
                    // DAR == W:H is square
                    display_aspect_ratio
                        != format!("{}:{}", video_stream.width, video_stream.height)
                }
            }
            None => false, // assumed SAR of 1:1
        }
    }

    fn needs_hw_device(&self) -> Option<HardwareAccel> {
        let encoder_needs = match self.output_context.video_codec {
            VideoCodec::H264Nvenc | VideoCodec::HevcNvenc => Some(HardwareAccel::Cuda),
            VideoCodec::H264Qsv | VideoCodec::HevcQsv => Some(HardwareAccel::Qsv),
            _ => None,
        };

        if encoder_needs.is_some() {
            return encoder_needs;
        }

        for filter in &self.filter_chain.filters {
            if let PipelineFilter::Video(video_filter) = filter
                && let Some(FrameSurface::Cuda) = video_filter.required_surface()
            {
                return Some(HardwareAccel::Cuda);
            }
        }

        None
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
    Pipeline::full(input_settings, output_settings)
}
