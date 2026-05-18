//! Cross-platform path resolution for the yt-dlp GUI.
//!
//! Centralizes the few directories the application touches:
//! - bundled sidecar binaries (`yt-dlp`, `ffmpeg`, `ffprobe`) shipped via
//!   Tauri 2 `bundle.externalBin`,
//! - the user's default downloads directory,
//! - a private temp directory for in-flight downloads,
//! - the user's config directory for persistent settings.
//!
//! Sidecar layout (set in `tauri.conf.json`):
//! - Dev (`cargo run` / `tauri dev`): `<manifest>/bin/<stem>-<TARGET_TRIPLE>[.exe]`.
//! - Bundled: Tauri strips the triple suffix and places the file next to the
//!   main executable (`<exe-dir>/<stem>[.exe]` on every supported OS).

#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Target triple this build was compiled for (e.g. `aarch64-apple-darwin`).
/// Emitted by `build.rs`. Used to locate dev-mode sidecar files which still
/// carry the triple suffix.
const TARGET_TRIPLE: &str = env!("TARGET_TRIPLE");

/// Returns the absolute path to the bundled yt-dlp sidecar binary.
///
/// The function does not check whether the file exists at the bundled
/// location; callers handle missing-binary errors at spawn time.
pub fn yt_dlp_binary_path() -> Result<PathBuf> {
    sidecar_path("yt-dlp").context("failed to resolve yt-dlp sidecar path")
}

/// Returns the absolute path to the bundled ffmpeg sidecar, or `None` if no
/// candidate exists (bundled location missing AND no PATH match).
pub fn ffmpeg_binary_path() -> Option<PathBuf> {
    locate_sidecar_or_path("ffmpeg")
}

/// Returns the absolute path to the bundled ffprobe sidecar, or `None`.
pub fn ffprobe_binary_path() -> Option<PathBuf> {
    locate_sidecar_or_path("ffprobe")
}

/// Resolve the canonical sidecar path for `stem` without verifying existence.
///
/// Returns the bundled-runtime path (`<exe-dir>/<stem>[.exe]`) when running
/// from an installed bundle, or the dev-mode path
/// (`<manifest>/bin/<stem>-<TARGET_TRIPLE>[.exe]`) when only that file exists.
fn sidecar_path(stem: &str) -> Result<PathBuf> {
    let file_name = sidecar_file_name(stem);

    let exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let exe_dir = exe
        .parent()
        .context("current executable has no parent directory")?
        .to_path_buf();

    let bundled = exe_dir.join(&file_name);
    if bundled.exists() {
        return Ok(bundled);
    }

    #[cfg(debug_assertions)]
    {
        let dev = dev_sidecar_path(stem);
        if dev.exists() {
            return Ok(dev);
        }
    }

    // Return the production location even when missing — surfaces a useful
    // error message at spawn time.
    Ok(bundled)
}

/// Sidecar lookup with `PATH` fallback for optional helper binaries.
fn locate_sidecar_or_path(stem: &str) -> Option<PathBuf> {
    let file_name = sidecar_file_name(stem);

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled = exe_dir.join(&file_name);
            if bundled.exists() {
                return Some(bundled);
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        let dev = dev_sidecar_path(stem);
        if dev.exists() {
            return Some(dev);
        }
    }

    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(&file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// `<stem>` on Unix, `<stem>.exe` on Windows.
fn sidecar_file_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// `<manifest>/bin/<stem>-<TARGET_TRIPLE>[.exe]` — where the unbundled sidecar
/// lives during `cargo run` / `tauri dev`.
#[cfg(debug_assertions)]
fn dev_sidecar_path(stem: &str) -> PathBuf {
    let suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bin")
        .join(format!("{stem}-{TARGET_TRIPLE}{suffix}"))
}

/// Returns the user's preferred downloads directory.
///
/// Falls back to the home directory, then to the OS temp dir, so this function
/// is infallible and always returns a usable path.
pub fn default_downloads_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_else(std::env::temp_dir))
}

/// Returns the application's private temp directory, creating it if needed.
///
/// Located at `<system-temp>/yt-dlp-gui`. Persists across runs on purpose so a
/// crash mid-download leaves partial files for a potential resume.
pub fn temp_dir() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("yt-dlp-gui");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create temp dir {}", dir.display()))?;
    Ok(dir)
}

/// Returns the application's config directory, creating it if needed.
///
/// Located at `<dirs::config_dir()>/gl.LSA.yt-dlp`. Errors if the platform
/// does not expose a config directory (rare; effectively only on exotic
/// targets).
pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .map(|p| p.join("gl.LSA.yt-dlp"))
        .context("no config directory available on this platform")?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config dir {}", dir.display()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_downloads_dir_is_absolute() {
        let p = default_downloads_dir();
        assert!(
            !p.as_os_str().is_empty(),
            "default_downloads_dir returned empty path"
        );
        // On every supported OS the fallback chain ends in an absolute path
        // (download dir, home dir, or system temp dir are all absolute).
        assert!(
            p.is_absolute(),
            "default_downloads_dir should be absolute, got {}",
            p.display()
        );
    }

    #[test]
    fn temp_dir_exists_after_call() {
        let p = temp_dir().expect("temp_dir() must succeed");
        assert!(
            p.exists(),
            "temp_dir() must create the directory; missing: {}",
            p.display()
        );
        assert!(p.is_dir(), "temp_dir() must be a directory");
        assert!(p.ends_with("yt-dlp-gui"));
    }

    #[test]
    fn yt_dlp_binary_path_filename_is_correct() {
        let p = yt_dlp_binary_path().expect("yt_dlp_binary_path() must succeed in tests");
        let file_name = p
            .file_name()
            .and_then(|s| s.to_str())
            .expect("path must have a UTF-8 file name");
        // Either the bundled sidecar name (stripped triple) or the dev-mode
        // triple-suffixed name; both end with `yt-dlp` modulo `.exe`.
        let stem = file_name.trim_end_matches(".exe");
        assert!(
            stem == "yt-dlp" || stem == format!("yt-dlp-{TARGET_TRIPLE}"),
            "unexpected binary file name: {file_name}"
        );
    }

    #[test]
    fn config_dir_creates_and_exists() {
        // config_dir is available on macOS, Linux, and Windows in CI/dev envs.
        // Skip gracefully if the platform has no config dir at all.
        let Ok(p) = config_dir() else {
            return;
        };
        assert!(p.exists(), "config_dir must exist: {}", p.display());
        assert!(p.ends_with("gl.LSA.yt-dlp"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn debug_manifest_ffmpeg_binaries_are_findable() {
        let expected_ffmpeg = dev_sidecar_path("ffmpeg");
        let expected_ffprobe = dev_sidecar_path("ffprobe");

        assert!(
            expected_ffmpeg.exists(),
            "expected dev sidecar ffmpeg at {}",
            expected_ffmpeg.display()
        );
        assert!(
            expected_ffprobe.exists(),
            "expected dev sidecar ffprobe at {}",
            expected_ffprobe.display()
        );

        assert_eq!(
            ffmpeg_binary_path().as_deref(),
            Some(expected_ffmpeg.as_path()),
            "ffmpeg_binary_path should resolve dev sidecar ffmpeg"
        );
        assert_eq!(
            ffprobe_binary_path().as_deref(),
            Some(expected_ffprobe.as_path()),
            "ffprobe_binary_path should resolve dev sidecar ffprobe"
        );
    }
}
