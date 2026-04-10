use std::time::Duration;

use crate::pipeline::{KEYFRAME_INTERVAL_SECONDS, OutputContext, SEGMENT_SECONDS};
use crate::video_codec::VideoCodec;

#[derive(Debug)]
pub enum OutputFormat {
    Hls {
        playlist: String,
        segment_template: String,
    },
}

impl OutputFormat {
    pub(crate) fn as_arg(&self, output_context: &OutputContext) -> Vec<String> {
        let force_key_frames_expr = format!("expr:gte(t,n_forced*{KEYFRAME_INTERVAL_SECONDS})");
        let segment_seconds = format!("{SEGMENT_SECONDS}");
        let rounded_frame_rate = output_context
            .media_frame_rate
            .parsed_frame_rate
            .round_ties_even() as u32;

        // TODO: 1-second GOP for qsv
        let gop = format!("{}", rounded_frame_rate * KEYFRAME_INTERVAL_SECONDS);
        let keyint_min = format!("{}", rounded_frame_rate * KEYFRAME_INTERVAL_SECONDS);

        let mut args: Vec<&str> = Vec::new();

        match self {
            OutputFormat::Hls {
                segment_template, ..
            } => {
                match output_context.video_codec {
                    VideoCodec::Copy => {}
                    _ => {
                        args.extend(vec![
                            "-g",
                            &gop,
                            "-keyint_min",
                            &keyint_min,
                            "-force_key_frames",
                            &force_key_frames_expr,
                        ]);
                    }
                }

                args.extend(vec![
                    "-f",
                    "hls",
                    "-hls_time",
                    &segment_seconds,
                    "-hls_list_size",
                    "0",
                    "-segment_list_flags",
                    "+live",
                    "-hls_segment_filename",
                    segment_template,
                    "-hls_segment_type",
                    "mpegts",
                    "-hls_flags",
                    "program_date_time+omit_endlist+append_list+independent_segments",
                ]);

                match output_context.pts_offset {
                    Some(pts_offset) if pts_offset.duration > Duration::ZERO => {}
                    _ => args.extend(vec![
                        "-hls_segment_options",
                        "mpegts_flags=+initial_discontinuity",
                    ]),
                }
            }
        }

        args.into_iter().map(String::from).collect()
    }

    pub(crate) fn path(&self) -> String {
        match self {
            OutputFormat::Hls { playlist, .. } => playlist.clone(),
        }
    }
}
