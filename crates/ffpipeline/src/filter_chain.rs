use crate::audio_filter::AudioFilter;
use crate::pipeline::{FrameState, FrameSurface, HardwareAccel, PixelFormat};
use crate::video_filter::VideoFilter;

#[derive(Clone)]
pub(crate) enum PipelineFilter {
    Audio(AudioFilter),
    Video(VideoFilter),
}

#[derive(Clone)]
pub(crate) struct FilterChain {
    pub(crate) filters: Vec<PipelineFilter>,
    audio_label: String,
    video_label: String,
    complex_filter: String,
}

impl FilterChain {
    pub(crate) fn new(filters: Vec<PipelineFilter>) -> FilterChain {
        FilterChain {
            filters,
            audio_label: String::new(),
            video_label: String::new(),
            complex_filter: String::new(),
        }
    }

    /// Disables/drops all audio filters from the filter chain.
    pub(crate) fn disable_audio(&mut self) {
        self.filters
            .retain(|f| !matches!(f, PipelineFilter::Audio(_)));
    }

    /// Disables/drops all video filters from the filter chain.
    pub(crate) fn disable_video(&mut self) {
        self.filters
            .retain(|f| !matches!(f, PipelineFilter::Video(_)));
    }

    /// Optimizes the filter chain by passing the frame state through each filter.
    /// Filters will be dropped when the input state already matches the desired output state.
    pub(crate) fn evaluate(&mut self, initial_state: &FrameState) {
        let mut state = initial_state.to_owned();
        let mut active_filters = Vec::new();

        for filter in &self.filters {
            match filter {
                PipelineFilter::Audio(af) => {
                    if let Some(new_filter) = af.evaluate(&state) {
                        new_filter.apply_to(&mut state);
                        active_filters.push(PipelineFilter::Audio(new_filter));
                    }
                }
                PipelineFilter::Video(vf) => {
                    if let Some(new_filter) = vf.evaluate(&state) {
                        new_filter.apply_to(&mut state);
                        active_filters.push(PipelineFilter::Video(new_filter));
                    }
                }
            }
        }

        self.filters = active_filters;
    }

    pub(crate) fn resolve(
        &mut self,
        accel: Option<HardwareAccel>,
        initial_state: &FrameState,
        encoder_surface: &FrameSurface,
        encoder_pixel_format: &Option<PixelFormat>,
    ) {
        let mut resolved = Vec::new();
        let mut current_state = initial_state.clone();

        for filter in &self.filters {
            match filter {
                PipelineFilter::Video(video_filter) => {
                    let best = video_filter.best_for(accel);

                    if let Some(required) = best.required_surface()
                        && current_state.surface != required
                    {
                        if required == FrameSurface::System {
                            let target_pixel_format = match current_state.pixel_format.bit_depth() {
                                10 => PixelFormat::P010le,
                                _ => PixelFormat::Nv12,
                            };

                            resolved.push(PipelineFilter::Video(VideoFilter::HwDownload {
                                target_pixel_format,
                            }));
                        } else {
                            resolved.push(PipelineFilter::Video(VideoFilter::HwUpload {
                                target_surface: required.clone(),
                            }))
                        }
                    }

                    best.apply_to(&mut current_state);
                    resolved.push(PipelineFilter::Video(best));
                }
                PipelineFilter::Audio(_) => {
                    // not sure if we should actually apply audio filters to the state
                    // since they are separate in the real filter graph
                    //audio_filter.apply_to(&mut current_state);
                    resolved.push(filter.clone());
                }
            }
        }

        if current_state.surface != *encoder_surface {
            if *encoder_surface == FrameSurface::System {
                let target_pixel_format = match current_state.pixel_format.bit_depth() {
                    10 => PixelFormat::P010le,
                    _ => PixelFormat::Nv12,
                };

                resolved.push(PipelineFilter::Video(VideoFilter::HwDownload {
                    target_pixel_format,
                }));
            } else {
                resolved.push(PipelineFilter::Video(VideoFilter::HwUpload {
                    target_surface: encoder_surface.clone(),
                }));
            }
        }

        if let Some(pixel_format) = encoder_pixel_format
            && current_state.pixel_format != *pixel_format
        {
            match current_state.surface {
                FrameSurface::Cuda => {
                    resolved.push(PipelineFilter::Video(VideoFilter::FormatCuda {
                        format: pixel_format.to_owned(),
                    }));
                }
                FrameSurface::System => resolved.push(PipelineFilter::Video(VideoFilter::Format {
                    format: pixel_format.to_owned(),
                })),
            }
        }

        self.filters = resolved;
    }

    pub(crate) fn build(&mut self, audio_label: &str, video_label: &str) {
        self.audio_label = audio_label.to_owned();
        self.video_label = video_label.to_owned();

        let mut filter_chains: Vec<String> = Vec::new();

        // build filter chain
        let audio_filter_count = self
            .filters
            .iter()
            .filter(|f| matches!(f, PipelineFilter::Audio(_)))
            .count();

        if audio_filter_count > 0 {
            let mut filter_chain = String::new();

            filter_chain.push_str(&format!("[{}]", self.audio_label));

            let mut audio_chain: Vec<String> = Vec::new();

            for filter in self.filters.iter() {
                if let PipelineFilter::Audio(audio_filter) = filter
                    && let Some(arg) = audio_filter.as_arg()
                {
                    audio_chain.push(arg)
                }
            }

            filter_chain.push_str(&audio_chain.join(","));

            self.audio_label = String::from("[a]");
            filter_chain.push_str(&self.audio_label);

            filter_chains.push(filter_chain);
        }

        let video_filter_count = self
            .filters
            .iter()
            .filter(|f| matches!(f, PipelineFilter::Video(_)))
            .count();

        if video_filter_count > 0 {
            let mut filter_chain = String::new();

            filter_chain.push_str(&format!("[{}]", self.video_label));

            let mut video_chain: Vec<String> = Vec::new();

            for filter in self.filters.iter() {
                if let PipelineFilter::Video(video_filter) = filter
                    && let Some(arg) = video_filter.as_arg()
                {
                    video_chain.push(arg);
                }
            }

            filter_chain.push_str(&video_chain.join(","));

            self.video_label = String::from("[v]");
            filter_chain.push_str(&self.video_label);

            filter_chains.push(filter_chain)
        }

        self.complex_filter = filter_chains.join(";");
    }

    pub(crate) fn audio_label(&self) -> &str {
        &self.audio_label
    }

    pub(crate) fn video_label(&self) -> &str {
        &self.video_label
    }

    pub(crate) fn as_arg(&self) -> Vec<String> {
        if self.complex_filter.is_empty() {
            Vec::new()
        } else {
            vec![
                String::from("-filter_complex"),
                self.complex_filter.to_owned(),
            ]
        }
    }
}
