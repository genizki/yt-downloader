//! Build script.
//!
//! - Runs `tauri_build::build()`.
//! - Exports `TARGET_TRIPLE` so runtime code can locate sidecar binaries
//!   placed under `bin/<stem>-<triple>[.exe]` during development.
//!
//! Binaries (yt-dlp, ffmpeg, ffprobe) are NOT downloaded here. They must be
//! placed manually under `bin/` using the Tauri sidecar naming convention:
//!   bin/yt-dlp-<TARGET_TRIPLE>[.exe]
//!   bin/ffmpeg-<TARGET_TRIPLE>[.exe]
//!   bin/ffprobe-<TARGET_TRIPLE>[.exe]
//! Tauri picks the file matching the active target triple at bundle time.

fn main() {
    let target = std::env::var("TARGET").expect("TARGET env var set by cargo");
    println!("cargo:rustc-env=TARGET_TRIPLE={target}");
    println!("cargo:rerun-if-changed=build.rs");

    tauri_build::build();
}
