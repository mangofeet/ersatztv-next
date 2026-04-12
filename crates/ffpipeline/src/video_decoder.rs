use crate::hardware_accel::HardwareAccel;
use crate::output_settings::OutputSettings;
use crate::probe::ProbeResultVideoStream;

pub struct VideoDecoder {
    input_codec: String,
    accel: Option<HardwareAccel>,
}

impl VideoDecoder {
    pub fn new(
        video_stream: &ProbeResultVideoStream,
        output_settings: &OutputSettings,
    ) -> VideoDecoder {
        VideoDecoder {
            input_codec: video_stream.codec.to_owned(),
            accel: output_settings.accel,
        }
    }

    pub(crate) fn as_arg(&self) -> Vec<String> {
        let implicit_cuda = vec![String::from("-hwaccel_output_format"), String::from("cuda")];

        match (self.input_codec.as_str(), self.accel) {
            ("mpeg2video", Some(HardwareAccel::Cuda)) => implicit_cuda,
            ("h264", Some(HardwareAccel::Cuda)) => implicit_cuda,
            ("hevc", Some(HardwareAccel::Cuda)) => implicit_cuda,
            _ => Vec::new(),
        }
    }
}
