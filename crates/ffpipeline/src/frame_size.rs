use crate::pipeline::FrameState;

#[derive(Debug, Clone, PartialEq)]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

impl FrameSize {
    pub(crate) fn square_pixel_size(&self, frame_state: &FrameState) -> FrameSize {
        // TODO: check is anamorphic, use SAR to calc width/height

        let source_width = frame_state.size.width as f64;
        let source_height = frame_state.size.height as f64;
        let target_width = self.width as f64;
        let target_height = self.height as f64;

        let width_percent = target_width / source_width;
        let height_percent = target_height / source_height;
        let min_percent = f64::min(width_percent, height_percent);

        FrameSize {
            width: (source_width * min_percent).floor() as u32,
            height: (source_height * min_percent).floor() as u32,
        }
    }
}
