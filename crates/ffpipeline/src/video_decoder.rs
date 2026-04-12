use crate::output_settings::OutputSettings;
use crate::pipeline::{FrameSurface, HardwareAccel, PixelFormat};
use crate::probe::ProbeResultVideoStream;

pub enum VideoDecoder {
    None,
    Software,
    HardwareAccel { accel: HardwareAccel },
}

impl VideoDecoder {
    pub(crate) fn new(
        video_stream: &ProbeResultVideoStream,
        is_still_image: bool,
        output_settings: &OutputSettings,
    ) -> VideoDecoder {
        // stream copy should not have a decoder; still image should not use accel
        if output_settings.video_format.is_none() || is_still_image {
            return VideoDecoder::None;
        }

        match output_settings.accel {
            Some(accel) => {
                if Self::can_hw_decode(accel, &video_stream.codec, &video_stream.pix_fmt) {
                    VideoDecoder::HardwareAccel { accel }
                } else {
                    VideoDecoder::Software
                }
            }
            None => VideoDecoder::Software,
        }
    }

    pub(crate) fn is_hardware(&self, hardware_accel: HardwareAccel) -> bool {
        match self {
            VideoDecoder::HardwareAccel { accel } => accel == &hardware_accel,
            _ => false,
        }
    }

    pub(crate) fn output_surface(&self) -> FrameSurface {
        match self {
            VideoDecoder::None => FrameSurface::System,
            VideoDecoder::Software => FrameSurface::System,
            VideoDecoder::HardwareAccel { accel } => match accel {
                HardwareAccel::Cuda => FrameSurface::Cuda,
                // TODO: other accels
                _ => FrameSurface::System,
            },
        }
    }

    pub(crate) fn output_format(&self, source_pixel_format: &PixelFormat) -> PixelFormat {
        match self {
            VideoDecoder::None => source_pixel_format.clone(),
            VideoDecoder::Software => source_pixel_format.clone(),
            VideoDecoder::HardwareAccel { accel } => match accel {
                HardwareAccel::Cuda => match source_pixel_format.bit_depth() {
                    10 => PixelFormat::P010le,
                    _ => PixelFormat::Nv12,
                },
                _ => source_pixel_format.clone(),
            },
        }
    }

    pub(crate) fn as_arg(&self) -> Vec<String> {
        match self {
            VideoDecoder::None => Vec::new(),
            VideoDecoder::Software => Vec::new(),
            VideoDecoder::HardwareAccel { accel } => match accel {
                HardwareAccel::Cuda => {
                    vec![
                        String::from("-hwaccel"),
                        String::from("cuda"),
                        String::from("-hwaccel_output_format"),
                        String::from("cuda"),
                    ]
                }
                _ => Vec::new(),
            },
        }
    }

    fn can_hw_decode(accel: HardwareAccel, codec: &str, pix_fmt: &str) -> bool {
        let pixel_format = PixelFormat::parse(pix_fmt);
        match (accel, pixel_format.bit_depth()) {
            (HardwareAccel::Cuda, 10) => matches!(codec, "av1" | "hevc"),
            (HardwareAccel::Cuda, _) => matches!(codec, "av1" | "h264" | "hevc" | "mpeg2video"),
            _ => false,
        }
    }
}
