use crate::pipeline::FrameState;

#[derive(Clone)]
pub enum AudioFilter {
    Resample,
    Pad,
}

impl AudioFilter {
    /// Determines whether the filter is needed given the input frame state. If so, the filter
    /// and its output frame state will be returned.
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
