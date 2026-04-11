use crate::pipeline::FrameState;
use crate::video_filter::VideoFilter;

#[derive(Clone)]
pub(crate) enum PipelineFilter {
    Video(VideoFilter),
}

#[derive(Clone)]
pub(crate) struct FilterChain {
    filters: Vec<PipelineFilter>,
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

    pub(crate) fn optimize(&mut self, initial_state: &FrameState) {
        // optimize filter chain by passing state through each
        let mut state = initial_state.to_owned();
        let mut active_filters = Vec::new();

        for filter in &self.filters {
            match filter {
                PipelineFilter::Video(vf) => {
                    if let Some((new_filter, new_state)) = vf.evaluate(&state) {
                        state = new_state;
                        active_filters.push(PipelineFilter::Video(new_filter))
                    }
                }
            }
        }

        self.filters = active_filters;
    }

    pub(crate) fn build(&mut self, audio_label: &str, video_label: &str) {
        self.audio_label = audio_label.to_owned();
        self.video_label = video_label.to_owned();

        // build filter chain
        let video_filter_count = self
            .filters
            .iter()
            .filter(|f| matches!(f, PipelineFilter::Video(_)))
            .count();

        if video_filter_count > 0 {
            self.complex_filter
                .push_str(&format!("[{}]", self.video_label));

            let mut video_chain: Vec<String> = Vec::new();

            for filter in self.filters.iter() {
                match filter {
                    PipelineFilter::Video(video_filter) => {
                        if let Some(arg) = video_filter.as_arg() {
                            video_chain.push(arg);
                        }
                    }
                }
            }

            self.complex_filter.push_str(&video_chain.join(","));

            self.video_label = String::from("[v]");
            self.complex_filter.push_str(&self.video_label);
        }
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
