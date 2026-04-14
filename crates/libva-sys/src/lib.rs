use std::ffi::{c_int, c_void};

pub type VADisplay = *mut c_void;
pub type VAStatus = c_int;
pub type VAProfile = c_int;
pub type VAEntrypoint = c_int;

pub const VA_PROFILE_NONE: VAProfile = -1;
pub const VA_PROFILE_MPEG2_SIMPLE: VAProfile = 0;
pub const VA_PROFILE_MPEG2_MAIN: VAProfile = 1;
pub const VA_PROFILE_MPEG4_SIMPLE: VAProfile = 2;
pub const VA_PROFILE_MPEG4_ADVANCED_SIMPLE: VAProfile = 3;
pub const VA_PROFILE_MPEG4_MAIN: VAProfile = 4;
pub const VA_PROFILE_H264_MAIN: VAProfile = 6;
pub const VA_PROFILE_H264_HIGH: VAProfile = 7;
pub const VA_PROFILE_VC1_SIMPLE: VAProfile = 8;
pub const VA_PROFILE_VC1_MAIN: VAProfile = 9;
pub const VA_PROFILE_VC1_ADVANCED: VAProfile = 10;
pub const VA_PROFILE_H264_CONSTRAINED_BASELINE: VAProfile = 13;
pub const VA_PROFILE_HEVC_MAIN: VAProfile = 17;
pub const VA_PROFILE_HEVC_MAIN10: VAProfile = 18;
pub const VA_PROFILE_VP9_PROFILE0: VAProfile = 19;
pub const VA_PROFILE_VP9_PROFILE1: VAProfile = 20;
pub const VA_PROFILE_VP9_PROFILE2: VAProfile = 21;
pub const VA_PROFILE_VP9_PROFILE3: VAProfile = 22;
pub const VA_PROFILE_AV1_PROFILE0: VAProfile = 32;
pub const VA_PROFILE_AV1_PROFILE1: VAProfile = 33;
pub const VA_PROFILE_H264_HIGH10: VAProfile = 36;

pub const VA_ENTRYPOINT_VLD: VAEntrypoint = 1;
pub const VA_ENTRYPOINT_ENC_SLICE: VAEntrypoint = 6;
pub const VA_ENTRYPOINT_ENC_SLICE_LP: VAEntrypoint = 8;

pub const VA_STATUS_SUCCESS: VAStatus = 0;

#[cfg(target_os = "linux")]
mod ffi;

#[cfg(target_os = "linux")]
pub use ffi::*;
