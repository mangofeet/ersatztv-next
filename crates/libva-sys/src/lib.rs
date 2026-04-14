#[cfg(target_os = "linux")]
mod ffi;

#[cfg(target_os = "linux")]
pub use ffi::*;
