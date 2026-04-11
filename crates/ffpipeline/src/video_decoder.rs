use crate::hardware_accel::HardwareAccel;
use crate::output_settings::OutputSettings;
use crate::probe::ProbeResultVideoStream;

pub struct VideoDecoder {
    _input_codec: String,
    _accel: Option<HardwareAccel>,
}

impl VideoDecoder {
    pub fn new(
        video_stream: &ProbeResultVideoStream,
        output_settings: &OutputSettings,
    ) -> VideoDecoder {
        VideoDecoder {
            _input_codec: video_stream.codec.to_owned(),
            _accel: output_settings.accel,
        }
    }

    pub(crate) fn as_arg(&self) -> Vec<String> {
        Vec::new()
    }
}
