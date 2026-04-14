fn main() {
    #[cfg(target_os = "linux")]
    {
        let host = std::env::var("HOST").unwrap();
        let target = std::env::var("TARGET").unwrap();

        if host == target {
            pkg_config::probe_library("libva").unwrap();
            pkg_config::probe_library("libva-drm").unwrap();
        } else {
            // cross-compiling: just emit the link directives directly.
            // the cross toolchain's sysroot provides the libraries.
            println!("cargo:rustc-link-lib=va");
            println!("cargo:rustc-link-lib=va-drm");
        }
    }
}
