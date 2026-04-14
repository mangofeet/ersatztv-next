use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;

pub type VADisplay = *mut c_void;
pub type VAStatus = c_int;
pub type VAProfile = c_int;
pub type VAEntrypoint = c_int;

unsafe extern "C" {
    pub fn vaGetDisplayDRM(fd: c_int) -> VADisplay;

    pub fn vaInitialize(
        dpy: VADisplay,
        major_version: *mut c_int,
        minor_version: *mut c_int,
    ) -> VAStatus;

    pub fn vaTerminate(dpy: VADisplay) -> VAStatus;

    pub fn vaQueryVendorString(dpy: VADisplay) -> *const c_char;

    pub fn vaMaxNumProfiles(dpy: VADisplay) -> c_int;

    pub fn vaMaxNumEntrypoints(dpy: VADisplay) -> c_int;

    pub fn vaQueryConfigProfiles(
        dpy: VADisplay,
        profile_list: *mut VAProfile,
        num_profiles: *mut c_int,
    ) -> VAStatus;

    pub fn vaQueryConfigEntrypoints(
        dpy: VADisplay,
        profile: VAProfile,
        entrypoint_list: *mut VAEntrypoint,
        num_entrypoints: *mut c_int,
    ) -> VAStatus;
}
