fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "linux"
        && (pkg_config::probe_library("libva").is_err()
            || pkg_config::probe_library("libva-drm").is_err())
    {
        println!("cargo:rustc-link-lib=va");
        println!("cargo:rustc-link-lib=va-drm");
    }
}
