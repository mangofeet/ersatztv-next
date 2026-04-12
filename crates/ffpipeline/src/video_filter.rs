use crate::frame_size::FrameSize;
use crate::hardware_accel::HardwareAccel;
use crate::pipeline::{FrameState, FrameSurface};

#[derive(Clone)]
pub enum ForceOriginalAspectRatio {
    Increase,
    Decrease,
}

#[derive(Clone)]
pub enum VideoFilter {
    HwUpload {
        target_surface: FrameSurface,
    },
    HwDownload,

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

    ScaleCuda {
        size: Option<FrameSize>,
    },
}

impl VideoFilter {
    /// Determines whether the filter is needed given the input frame state.
    pub(crate) fn evaluate(&self, state: &FrameState) -> Option<VideoFilter> {
        match self {
            // hardware filters aren't present at the point where this is called
            VideoFilter::HwUpload { .. } => None,
            VideoFilter::HwDownload => None,
            VideoFilter::ScaleCuda { .. } => None,

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

                    Some(VideoFilter::Scale {
                        size: Some(actual.clone()),
                        input_is_anamorphic: state.is_anamorphic,
                        force_original_aspect_ratio,
                    })
                }
            }
            VideoFilter::Scale { size: None, .. } => None,
            VideoFilter::Pad { size: Some(target) } => {
                if state.size == *target {
                    None
                } else {
                    Some(self.clone())
                }
            }
            VideoFilter::Pad { size: None } => None,
            VideoFilter::Loop { codec } if codec == "png" => Some(self.clone()),
            VideoFilter::Loop { .. } => None,
        }
    }

    pub(crate) fn apply_to(&self, state: &mut FrameState) {
        match self {
            VideoFilter::HwUpload { target_surface } => {
                state.surface = target_surface.clone();
            }
            VideoFilter::HwDownload => {
                state.surface = FrameSurface::System;
            }
            VideoFilter::Scale {
                size: Some(size), ..
            } => {
                state.size = size.clone();
                state.is_anamorphic = false;
                state.sample_aspect_ratio = Some(String::from("1:1"));
                state.display_aspect_ratio = None;
            }
            VideoFilter::Pad { size: Some(size) } => {
                state.size = size.clone();
                state.surface = FrameSurface::System;
            }
            VideoFilter::ScaleCuda {
                size: Some(size), ..
            } => {
                state.size = size.clone();
                state.surface = FrameSurface::Cuda;
                // TODO: anamorphic handling
            }
            _ => {}
        }
    }

    pub(crate) fn best_for(&self, accel: Option<HardwareAccel>) -> VideoFilter {
        match (self, accel) {
            (VideoFilter::Scale { size, .. }, Some(HardwareAccel::Cuda)) => {
                VideoFilter::ScaleCuda { size: size.clone() }
            }
            _ => self.clone(),
        }
    }

    pub(crate) fn required_surface(&self) -> Option<FrameSurface> {
        match self {
            VideoFilter::HwUpload { .. } => None,
            VideoFilter::HwDownload => None,
            VideoFilter::Scale { .. } => Some(FrameSurface::System),
            VideoFilter::Pad { .. } => Some(FrameSurface::System),
            VideoFilter::Loop { .. } => Some(FrameSurface::System),

            VideoFilter::ScaleCuda { .. } => Some(FrameSurface::Cuda),
        }
    }

    pub(crate) fn as_arg(&self) -> Option<String> {
        match self {
            VideoFilter::HwUpload { target_surface } => match target_surface {
                FrameSurface::Cuda => Some(String::from("hwupload_cuda")),
                _ => None,
            },
            VideoFilter::HwDownload => Some(String::from("hwdownload,format=nv12")),
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
            VideoFilter::ScaleCuda { size: Some(size) } => Some(format!(
                // TODO: anamorphic, aspect ratio
                "scale_cuda={}:{}",
                size.width, size.height
            )),
            VideoFilter::ScaleCuda { .. } => None,
        }
    }
}
