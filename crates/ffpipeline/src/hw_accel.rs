use crate::accel;
use crate::ffmpeg_info::FfmpegInfo;
use crate::pipeline::{FrameSurface, PixelFormat, VideoFormat};
use crate::video_codec::VideoCodec;
use crate::video_filter::VideoFilter;

pub trait HwAccel {
    fn best_filter(&self, video_filter: &VideoFilter, ffmpeg_info: &FfmpegInfo) -> VideoFilter;
    fn can_decode(&self, codec: &str, pixel_format: &PixelFormat) -> bool;
    fn codec_for_format(&self, format: &VideoFormat) -> VideoCodec;
    fn decoder_arg(&self) -> Vec<String>;
    fn ffmpeg_name(&self) -> &str;
    fn frame_surface(&self) -> FrameSurface;
    fn init_hw_device(&self) -> Vec<String>;
    fn output_format(&self, source_pixel_format: &PixelFormat) -> PixelFormat;
}

#[derive(Debug, Clone, PartialEq)]
pub enum HardwareAccel {
    Cuda(accel::cuda::Cuda),
    Qsv(accel::qsv::Qsv),
    Vaapi(accel::vaapi::Vaapi),
    VideoToolbox(accel::video_toolbox::VideoToolbox),
}

impl HwAccel for HardwareAccel {
    fn best_filter(&self, video_filter: &VideoFilter, ffmpeg_info: &FfmpegInfo) -> VideoFilter {
        match self {
            Self::Cuda(a) => a.best_filter(video_filter, ffmpeg_info),
            Self::Qsv(a) => a.best_filter(video_filter, ffmpeg_info),
            Self::Vaapi(a) => a.best_filter(video_filter, ffmpeg_info),
            Self::VideoToolbox(a) => a.best_filter(video_filter, ffmpeg_info),
        }
    }

    fn can_decode(&self, codec: &str, pixel_format: &PixelFormat) -> bool {
        match self {
            Self::Cuda(a) => a.can_decode(codec, pixel_format),
            Self::Qsv(a) => a.can_decode(codec, pixel_format),
            Self::Vaapi(a) => a.can_decode(codec, pixel_format),
            Self::VideoToolbox(a) => a.can_decode(codec, pixel_format),
        }
    }

    fn codec_for_format(&self, format: &VideoFormat) -> VideoCodec {
        match self {
            Self::Cuda(a) => a.codec_for_format(format),
            Self::Qsv(a) => a.codec_for_format(format),
            Self::Vaapi(a) => a.codec_for_format(format),
            Self::VideoToolbox(a) => a.codec_for_format(format),
        }
    }

    fn decoder_arg(&self) -> Vec<String> {
        match self {
            Self::Cuda(a) => a.decoder_arg(),
            Self::Qsv(a) => a.decoder_arg(),
            Self::Vaapi(a) => a.decoder_arg(),
            Self::VideoToolbox(a) => a.decoder_arg(),
        }
    }

    fn ffmpeg_name(&self) -> &str {
        match self {
            Self::Cuda(a) => a.ffmpeg_name(),
            Self::Qsv(a) => a.ffmpeg_name(),
            Self::Vaapi(a) => a.ffmpeg_name(),
            Self::VideoToolbox(a) => a.ffmpeg_name(),
        }
    }
    fn frame_surface(&self) -> FrameSurface {
        match self {
            Self::Cuda(a) => a.frame_surface(),
            Self::Qsv(a) => a.frame_surface(),
            Self::Vaapi(a) => a.frame_surface(),
            Self::VideoToolbox(a) => a.frame_surface(),
        }
    }

    fn init_hw_device(&self) -> Vec<String> {
        match self {
            Self::Cuda(a) => a.init_hw_device(),
            Self::Qsv(a) => a.init_hw_device(),
            Self::Vaapi(a) => a.init_hw_device(),
            Self::VideoToolbox(a) => a.init_hw_device(),
        }
    }

    fn output_format(&self, source_pixel_format: &PixelFormat) -> PixelFormat {
        match self {
            Self::Cuda(a) => a.output_format(source_pixel_format),
            Self::Qsv(a) => a.output_format(source_pixel_format),
            Self::Vaapi(a) => a.output_format(source_pixel_format),
            Self::VideoToolbox(a) => a.output_format(source_pixel_format),
        }
    }
}
