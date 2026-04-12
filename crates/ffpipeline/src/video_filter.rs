use crate::frame_size::FrameSize;
use crate::pipeline::FrameState;

#[derive(Clone)]
pub enum ForceOriginalAspectRatio {
    Increase,
    Decrease,
}

#[derive(Clone)]
pub enum VideoFilter {
    Scale {
        size: Option<FrameSize>,
        input_is_anamorphic: bool,
        force_original_aspect_ratio: Option<ForceOriginalAspectRatio>,
    },
    Pad {
        size: Option<FrameSize>,
    },
    Loop {
        codec: String,
    },
}

impl VideoFilter {
    pub(crate) fn evaluate(&self, state: &FrameState) -> Option<(VideoFilter, FrameState)> {
        match self {
            VideoFilter::Scale {
                size: Some(target), ..
            } => {
                if state.size == *target {
                    None
                } else {
                    let actual = target.square_pixel_size(state);
                    let force_original_aspect_ratio = if actual == *target {
                        None
                    } else {
                        Some(ForceOriginalAspectRatio::Decrease)
                    };

                    Some((
                        VideoFilter::Scale {
                            size: Some(actual.clone()),
                            input_is_anamorphic: state.is_anamorphic,
                            force_original_aspect_ratio,
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
            VideoFilter::Scale { size: None, .. } => None,
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
            VideoFilter::Scale {
                size: Some(size),
                input_is_anamorphic,
                force_original_aspect_ratio,
            } => {
                let aspect_ratio = match force_original_aspect_ratio {
                    Some(ForceOriginalAspectRatio::Increase) => {
                        String::from(":force_original_aspect_ratio=increase")
                    }
                    Some(ForceOriginalAspectRatio::Decrease) => {
                        String::from(":force_original_aspect_ratio=decrease")
                    }
                    None => String::new(),
                };

                if *input_is_anamorphic {
                    Some(format!(
                        "scale=iw*sar:ih,setsar=1,scale={}:{}:flags=fast_bilinear{}",
                        size.width, size.height, aspect_ratio
                    ))
                } else {
                    Some(format!(
                        "scale={}:{}:flags=fast_bilinear{},setsar=1",
                        size.width, size.height, aspect_ratio
                    ))
                }
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
