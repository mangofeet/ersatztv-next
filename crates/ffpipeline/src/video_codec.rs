use crate::pipeline::PixelFormat;

#[derive(Copy, Clone, PartialEq)]
pub enum VideoCodec {
    Copy,
    H264Nvenc,
    HevcNvenc,
    H264Qsv,
    HevcQsv,
    H264VideoToolbox,
    HevcVideoToolbox,
    Libx264,
    Libx265,
}

impl VideoCodec {
    pub(crate) fn preferred_pixel_format(&self, bit_depth: u32) -> Option<PixelFormat> {
        match self {
            VideoCodec::H264Nvenc => Some(PixelFormat::Nv12),
            VideoCodec::HevcNvenc => match bit_depth {
                10 => Some(PixelFormat::P010le),
                _ => Some(PixelFormat::Nv12),
            },
            _ => None,
        }
    }

    pub(crate) fn as_arg(&self) -> Vec<String> {
        let codec: &str = match self {
            VideoCodec::Copy => "copy",
            VideoCodec::H264Nvenc => "h264_nvenc",
            VideoCodec::HevcNvenc => "hevc_nvenc",
            VideoCodec::H264Qsv => "h264_qsv",
            VideoCodec::HevcQsv => "hevc_qsv",
            VideoCodec::H264VideoToolbox => "h264_videotoolbox",
            VideoCodec::HevcVideoToolbox => "hevc_videotoolbox",
            VideoCodec::Libx264 => "libx264",
            VideoCodec::Libx265 => "libx265",
        };

        let options = match self {
            VideoCodec::Libx265 => vec!["-tag:v", "hvc1", "-x265-params", "log-level=error"],
            VideoCodec::HevcQsv => vec!["-low_power", "0", "-look_ahead", "0"],
            _ => Vec::new(),
        };

        [&["-vcodec", codec], &options[..]]
            .concat()
            .into_iter()
            .map(String::from)
            .collect()
    }
}
