//! Cross-platform path resolution for the yt-dlp GUI.
//!
//! Centralizes the few directories the application touches:
//! - the bundled `yt-dlp` binary (per-OS subdir under `<exe-dir>/bin/yt-dlp/`),
//! - the user's default downloads directory,
//! - a private temp directory for in-flight downloads,
//! - the user's config directory for persistent settings.

// Public path helpers are consumed by sibling modules that are still
// in-progress (settings, download manager, app bootstrap). Suppress the
// `dead_code` lint at the module level until those callers land.
#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Returns the absolute path to the bundled yt-dlp binary for the current OS.
///
/// Layout: `<exe-dir>/bin/yt-dlp/<os>/yt-dlp[.exe]`, where `<os>` is one of
/// `windows`, `macos`, `linux`. The `.exe` suffix is appended only on Windows.
///
/// The function does not check whether the file exists — callers may run before
/// `build.rs` has had a chance to download it, and the existence check belongs
/// to the spawn site.
pub fn yt_dlp_binary_path() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let exe_dir = exe
        .parent()
        .context("current executable has no parent directory")?
        .to_path_buf();

    let os_subdir = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        // All non-windows, non-macos targets are treated as linux for the
        // bundled-binary layout. This keeps the path structure predictable.
        "linux"
    };

    let file_name = if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };

    Ok(exe_dir
        .join("bin")
        .join("yt-dlp")
        .join(os_subdir)
        .join(file_name))
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
/// Located at `<dirs::config_dir()>/yt-dlp-gui`. Errors if the platform does
/// not expose a config directory (rare; effectively only on exotic targets).
pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .map(|p| p.join("yt-dlp-gui"))
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
        assert!(
            file_name == "yt-dlp" || file_name == "yt-dlp.exe",
            "unexpected binary file name: {file_name}"
        );

        // The path should also live under .../bin/yt-dlp/<os>/<file>.
        let components: Vec<_> = p
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        assert!(
            components.iter().any(|c| c == "bin"),
            "expected `bin` in path: {}",
            p.display()
        );
        assert!(
            components.iter().any(|c| c == "yt-dlp"),
            "expected `yt-dlp` in path: {}",
            p.display()
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
        assert!(p.ends_with("yt-dlp-gui"));
    }
}
