use crate::ffmpeg_info::{FfmpegInfo, KnownVideoFilter};
use crate::hw_accel::HwAccel;
use crate::pipeline::{FrameSurface, PixelFormat, VideoFormat};
use crate::video_codec::VideoCodec;
use crate::video_filter::VideoFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct Vaapi {
    pub device: String,
}

impl HwAccel for Vaapi {
    fn best_filter(&self, video_filter: &VideoFilter, ffmpeg_info: &FfmpegInfo) -> VideoFilter {
        match video_filter {
            VideoFilter::Scale {
                size,
                input_is_anamorphic,
                force_original_aspect_ratio,
            } if ffmpeg_info.has_video_filter(&KnownVideoFilter::ScaleVaapi) => {
                VideoFilter::ScaleVaapi {
                    size: size.clone(),
                    input_is_anamorphic: *input_is_anamorphic,
                    force_original_aspect_ratio: force_original_aspect_ratio.clone(),
                }
            }
            VideoFilter::Pad { size }
                if ffmpeg_info.has_video_filter(&KnownVideoFilter::PadVaapi) =>
            {
                VideoFilter::PadVaapi { size: size.clone() }
            }
            _ => video_filter.clone(),
        }
    }

    fn can_decode(&self, codec: &str, pixel_format: &PixelFormat) -> bool {
        match pixel_format.bit_depth() {
            10 => matches!(codec, "hevc"),
            8 => matches!(codec, "h264" | "hevc" | "mpeg2video"),
            _ => false,
        }
    }

    fn codec_for_format(&self, format: &VideoFormat) -> VideoCodec {
        match format {
            VideoFormat::H264 => VideoCodec {
                codec_name: "h264_vaapi",
                options: &[],
                preferred_pixel_format_8bit: Some(PixelFormat::Nv12),
                preferred_pixel_format_10bit: Some(PixelFormat::P010le),
                is_hardware: true,
            },
            VideoFormat::Hevc => VideoCodec {
                codec_name: "hevc_vaapi",
                options: &[],
                preferred_pixel_format_8bit: Some(PixelFormat::Nv12),
                preferred_pixel_format_10bit: Some(PixelFormat::P010le),
                is_hardware: true,
            },
        }
    }

    fn decoder_arg(&self) -> Vec<String> {
        vec![
            String::from("-hwaccel"),
            String::from("vaapi"),
            String::from("-vaapi_device"),
            self.device.clone(),
            String::from("-hwaccel_output_format"),
            String::from("vaapi"),
        ]
    }

    fn ffmpeg_name(&self) -> &str {
        "vaapi"
    }

    fn frame_surface(&self) -> FrameSurface {
        FrameSurface::Vaapi
    }

    fn init_hw_device(&self) -> Vec<String> {
        Vec::new()
    }

    fn output_format(&self, source_pixel_format: &PixelFormat) -> PixelFormat {
        match source_pixel_format.bit_depth() {
            10 => PixelFormat::P010le,
            _ => PixelFormat::Nv12,
        }
    }
}
