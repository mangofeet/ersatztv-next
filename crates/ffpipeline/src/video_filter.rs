use crate::ffmpeg_info::{FfmpegInfo, KnownVideoFilter};
use crate::frame_size::FrameSize;
use crate::pipeline::{FrameState, FrameSurface, HardwareAccel, PixelFormat};

#[derive(Clone)]
pub enum ForceOriginalAspectRatio {
    Increase,
    Decrease,
}

impl ForceOriginalAspectRatio {
    fn as_arg(&self) -> String {
        match self {
            ForceOriginalAspectRatio::Increase => {
                String::from(":force_original_aspect_ratio=increase")
            }
            ForceOriginalAspectRatio::Decrease => {
                String::from(":force_original_aspect_ratio=decrease")
            }
        }
    }
}

#[derive(Clone)]
pub enum VideoFilter {
    HwUpload {
        target_surface: FrameSurface,
    },
    HwDownload {
        target_pixel_format: PixelFormat,
    },
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
    Format {
        format: PixelFormat,
    },
    ScaleCuda {
        size: Option<FrameSize>,
        input_is_anamorphic: bool,
        force_original_aspect_ratio: Option<ForceOriginalAspectRatio>,
    },
    FormatCuda {
        format: PixelFormat,
    },
    PadCuda {
        size: Option<FrameSize>,
    },
    CudaHwUploadFallback {
        target_pixel_format: Option<PixelFormat>,
    },
    ScaleQsv {
        size: Option<FrameSize>,
    },
}

impl VideoFilter {
    /// Determines whether the filter is needed given the input frame state.
    pub(crate) fn evaluate(&self, state: &FrameState) -> Option<VideoFilter> {
        match self {
            // hardware filters aren't present at the point where this is called
            VideoFilter::HwUpload { .. } => None,
            VideoFilter::HwDownload { .. } => None,
            VideoFilter::ScaleCuda { .. } => None,
            VideoFilter::FormatCuda { .. } => None,
            VideoFilter::PadCuda { .. } => None,
            VideoFilter::ScaleQsv { .. } => None,
            VideoFilter::Format { .. } => None,

            VideoFilter::CudaHwUploadFallback { .. } => {
                if state.surface == FrameSurface::Cuda {
                    Some(self.clone())
                } else {
                    None
                }
            }

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
            VideoFilter::HwDownload {
                target_pixel_format,
            } => {
                state.surface = FrameSurface::System;
                state.pixel_format = target_pixel_format.clone();
            }
            VideoFilter::Scale {
                size: Some(size), ..
            } => {
                state.size = size.clone();
                state.is_anamorphic = false;
                state.sample_aspect_ratio = Some(String::from("1:1"));
                state.display_aspect_ratio = None;
            }
            VideoFilter::Scale { size: None, .. } => {}
            VideoFilter::Pad { size: Some(size) } => {
                state.size = size.clone();
                state.surface = FrameSurface::System;
            }
            VideoFilter::Pad { size: None } => {}
            VideoFilter::Loop { .. } => {}
            VideoFilter::Format { format } => {
                state.pixel_format = format.clone();
            }
            VideoFilter::ScaleCuda {
                size: Some(size), ..
            } => {
                state.size = size.clone();
                state.surface = FrameSurface::Cuda;
                state.is_anamorphic = false;
                state.sample_aspect_ratio = Some(String::from("1:1"));
                state.display_aspect_ratio = None;
            }
            VideoFilter::ScaleCuda { size: None, .. } => {}
            VideoFilter::FormatCuda { format } => {
                state.pixel_format = format.clone();
            }
            VideoFilter::PadCuda { size: Some(size) } => {
                state.size = size.clone();
                state.surface = FrameSurface::Cuda;
            }
            VideoFilter::PadCuda { size: None } => {}
            VideoFilter::CudaHwUploadFallback {
                target_pixel_format: Some(format),
            } => {
                state.pixel_format = format.clone();
            }
            VideoFilter::CudaHwUploadFallback {
                target_pixel_format: None,
            } => {}
            VideoFilter::ScaleQsv { size: Some(size) } => {
                state.size = size.clone();
                state.surface = FrameSurface::Qsv;
                // TODO: anamorphic handling
            }
            VideoFilter::ScaleQsv { size: None } => {}
        }
    }

    pub(crate) fn best_for(
        &self,
        accel: Option<HardwareAccel>,
        ffmpeg_info: &FfmpegInfo,
    ) -> VideoFilter {
        match (self, accel) {
            (
                VideoFilter::Scale {
                    size,
                    input_is_anamorphic,
                    force_original_aspect_ratio,
                },
                Some(HardwareAccel::Cuda),
            ) => VideoFilter::ScaleCuda {
                size: size.clone(),
                input_is_anamorphic: *input_is_anamorphic,
                force_original_aspect_ratio: force_original_aspect_ratio.clone(),
            },
            (VideoFilter::Scale { size, .. }, Some(HardwareAccel::Qsv)) => {
                if ffmpeg_info.has_video_filter(&KnownVideoFilter::VppQsv) {
                    VideoFilter::ScaleQsv {
                        size: size.clone(),
                        //input_is_anamorphic: *input_is_anamorphic,
                        //force_original_aspect_ratio: force_original_aspect_ratio.clone(),
                    }
                } else {
                    self.clone()
                }
            }
            (VideoFilter::Pad { size }, Some(HardwareAccel::Cuda)) => {
                if ffmpeg_info.has_video_filter(&KnownVideoFilter::PadCuda) {
                    VideoFilter::PadCuda { size: size.clone() }
                } else {
                    self.clone()
                }
            }
            _ => self.clone(),
        }
    }

    pub(crate) fn required_surface(&self) -> Option<FrameSurface> {
        match self {
            VideoFilter::HwUpload { .. } => None,
            VideoFilter::HwDownload { .. } => None,
            VideoFilter::Scale { .. } => Some(FrameSurface::System),
            VideoFilter::Pad { .. } => Some(FrameSurface::System),
            VideoFilter::Loop { .. } => Some(FrameSurface::System),
            VideoFilter::Format { .. } => Some(FrameSurface::System),

            VideoFilter::ScaleCuda { .. } => Some(FrameSurface::Cuda),
            VideoFilter::FormatCuda { .. } => Some(FrameSurface::Cuda),
            VideoFilter::PadCuda { .. } => Some(FrameSurface::Cuda),
            VideoFilter::CudaHwUploadFallback { .. } => None,

            VideoFilter::ScaleQsv { .. } => Some(FrameSurface::Qsv),
        }
    }

    pub(crate) fn as_arg(&self) -> Option<String> {
        match self {
            VideoFilter::HwUpload { target_surface } => match target_surface {
                FrameSurface::Cuda => Some(String::from("hwupload_cuda")),
                FrameSurface::Qsv => Some(String::from("hwupload=extra_hw_frames=64")),
                _ => None,
            },
            VideoFilter::HwDownload {
                target_pixel_format,
            } => Some(format!(
                "hwdownload,format={}",
                target_pixel_format.as_arg()
            )),
            VideoFilter::Scale {
                size: Some(size),
                input_is_anamorphic,
                force_original_aspect_ratio,
            } => {
                let aspect_ratio = force_original_aspect_ratio
                    .as_ref()
                    .map_or(String::new(), |f| f.as_arg());

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
            VideoFilter::Format { format } => Some(format!("format={}", format.as_arg())),
            VideoFilter::ScaleCuda {
                size: Some(size),
                input_is_anamorphic,
                force_original_aspect_ratio,
            } => {
                let aspect_ratio = force_original_aspect_ratio
                    .as_ref()
                    .map_or(String::new(), |f| f.as_arg());

                if *input_is_anamorphic {
                    Some(format!(
                        "scale_cuda=iw*sar:ih,setsar=1,scale_cuda={}:{}{}",
                        size.width, size.height, aspect_ratio
                    ))
                } else {
                    Some(format!(
                        "scale_cuda={}:{}{},setsar=1",
                        size.width, size.height, aspect_ratio
                    ))
                }
            }
            VideoFilter::ScaleCuda { .. } => None,
            VideoFilter::FormatCuda { format } => {
                Some(format!("scale_cuda=format={}", format.as_arg()))
            }
            VideoFilter::PadCuda { size: Some(size) } => Some(format!(
                "pad_cuda={}:{}:-1:-1:color=black,setsar=1",
                size.width, size.height
            )),
            VideoFilter::PadCuda { size: None } => None,
            VideoFilter::CudaHwUploadFallback {
                target_pixel_format: Some(_format),
            } => Some(String::from("hwupload")),
            VideoFilter::CudaHwUploadFallback {
                target_pixel_format: None,
            } => None,
            VideoFilter::ScaleQsv { size: Some(size) } => {
                // TODO: anamorphic handling
                Some(format!("vpp_qsv=w={}:h={}", size.width, size.height))
            }
            VideoFilter::ScaleQsv { size: None } => None,
        }
    }
}
