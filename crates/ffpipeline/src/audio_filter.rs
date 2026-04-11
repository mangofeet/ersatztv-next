use crate::pipeline::FrameState;

#[derive(Clone)]
pub enum AudioFilter {
    Resample,
    Pad,
}

impl AudioFilter {
    pub(crate) fn evaluate(&self, state: &FrameState) -> Option<(AudioFilter, FrameState)> {
        Some((self.clone(), state.clone()))
    }

    pub(crate) fn as_arg(&self) -> Option<String> {
        match self {
            AudioFilter::Resample => Some(String::from("aresample=async=1")),
            AudioFilter::Pad => Some(String::from("apad")),
        }
    }
}
