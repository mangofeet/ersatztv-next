use crate::frame_size::FrameSize;
use crate::pipeline::FrameState;

#[derive(Clone)]
pub enum VideoFilter {
    Scale { size: Option<FrameSize> },
    Pad { size: Option<FrameSize> },
    Loop { codec: String },
}

impl VideoFilter {
    pub(crate) fn evaluate(&self, state: &FrameState) -> Option<(VideoFilter, FrameState)> {
        match self {
            VideoFilter::Scale { size: Some(target) } => {
                if state.size == *target {
                    None
                } else {
                    let actual = target.square_pixel_size(state);
                    Some((
                        VideoFilter::Scale {
                            size: Some(actual.clone()),
                        },
                        FrameState {
                            size: actual,
                            is_anamorphic: false,
                            sample_aspect_ratio: Some(String::from("1:1")),
                            display_aspect_ratio: None,
                        },
                    ))
                }
            }
            VideoFilter::Scale { size: None } => None,
            VideoFilter::Pad { size: Some(target) } => {
                if state.size == *target {
                    None
                } else {
                    // pad will always result in the proper dimensions
                    Some((
                        self.clone(),
                        FrameState {
                            size: target.clone(),
                            is_anamorphic: false,
                            sample_aspect_ratio: Some(String::from("1:1")),
                            display_aspect_ratio: None,
                        },
                    ))
                }
            }
            VideoFilter::Pad { size: None } => None,
            VideoFilter::Loop { codec } if codec == "png" => Some((self.clone(), state.clone())),
            VideoFilter::Loop { .. } => None,
        }
    }

    pub(crate) fn as_arg(&self) -> Option<String> {
        match self {
            VideoFilter::Scale { size: Some(size) } => {
                Some(format!("scale=w={}:h={}", size.width, size.height))
            }
            VideoFilter::Scale { .. } => None,
            VideoFilter::Pad { size: Some(size) } => Some(format!(
                "pad={}:{}:-1:-1:color=black",
                size.width, size.height
            )),
            VideoFilter::Pad { .. } => None,
            VideoFilter::Loop { .. } => Some(String::from("loop=-1:1")),
        }
    }
}
