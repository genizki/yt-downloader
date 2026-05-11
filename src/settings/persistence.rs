//! TOML persistence for `AppSettings`.
//!
//! On-disk location: `<config_dir>/settings.toml` (config_dir provided by
//! `crate::paths::config_dir()`, which already pins the per-app subdir).
//!
//! `load()` is intentionally infallible from the caller's POV: a missing
//! file or any parse error returns `Default` and emits `tracing::warn!`.
//! `save()` performs an atomic write (write to a sibling `.tmp` file, then
//! `rename` over the target) so a crash mid-write cannot leave a half-baked
//! settings file behind.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::model::AppSettings;

/// Returns the canonical settings file path: `<config_dir>/settings.toml`.
pub fn settings_path() -> Result<PathBuf> {
    Ok(crate::paths::config_dir()?.join("settings.toml"))
}

/// Loads settings from disk. Returns `Default` on any error (file missing,
/// malformed TOML, schema mismatch, ...) and emits a warning trace.
pub fn load() -> AppSettings {
    match settings_path() {
        Ok(p) => load_from(&p),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "could not resolve settings path; falling back to defaults"
            );
            AppSettings::default()
        }
    }
}

/// Persists settings to disk atomically. Errors propagate so the UI can
/// surface them (write-permission issues, full disks, ...).
pub fn save(s: &AppSettings) -> Result<()> {
    save_to(&settings_path()?, s)
}

/// Internal: `load()` against an arbitrary path. Useful for tests so they
/// don't have to share the global config dir between cases.
pub(crate) fn load_from(path: &Path) -> AppSettings {
    match std::fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<AppSettings>(&text) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %path.display(),
                    "settings.toml parse error; falling back to defaults"
                );
                AppSettings::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // First run / fresh install — perfectly normal, log at debug.
            tracing::debug!(
                path = %path.display(),
                "settings.toml not found; using defaults"
            );
            AppSettings::default()
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "could not read settings.toml; falling back to defaults"
            );
            AppSettings::default()
        }
    }
}

/// Internal: atomic `save()` against an arbitrary path. Writes to a sibling
/// `.tmp` file in the same directory and `rename`s on top of `path`; the
/// rename is atomic on POSIX and on NTFS (when source/dest live on the same
/// volume), so a partial write can never become the new settings file.
pub(crate) fn save_to(path: &Path, s: &AppSettings) -> Result<()> {
    let serialized = toml::to_string_pretty(s).context("failed to serialize settings to TOML")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create settings dir {}", parent.display()))?;
    }

    let tmp = tmp_sibling(path);
    std::fs::write(&tmp, serialized.as_bytes())
        .with_context(|| format!("failed to write temp settings file {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("failed to rename {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Builds the sibling temp path used by the atomic-write dance. Keeping the
/// temp file in the same directory guarantees `rename` stays on one volume
/// (cross-device renames would fall back to a non-atomic copy+delete).
fn tmp_sibling(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::Extras;

    /// Tiny temp-dir helper. Returns a unique directory under the system
    /// temp root. Callers leak the directory on purpose — settings files
    /// are tiny and the OS cleans `/tmp` itself.
    fn fresh_temp_dir(tag: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "yt-dlp-gui-tests-{tag}-{nanos}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create test temp dir");
        dir
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = fresh_temp_dir("roundtrip");
        let path = dir.join("settings.toml");

        let original = AppSettings {
            language: "de".into(),
            auth_token: "secret-token".into(),
            youtube_api_key: "AIza-test-key".into(),
            playlist_auto_download: true,
            extras: Extras {
                embed_chapters: true,
                ..AppSettings::default().extras
            },
            ..AppSettings::default()
        };

        save_to(&path, &original).expect("save_to should succeed");
        assert!(path.exists(), "settings.toml should exist after save_to");

        let loaded = load_from(&path);
        assert_eq!(loaded, original, "round-tripped settings must be equal");
    }

    #[test]
    fn load_returns_default_when_missing() {
        let dir = fresh_temp_dir("missing");
        let path = dir.join("does-not-exist.toml");
        assert!(!path.exists(), "precondition: file must not exist");

        let s = load_from(&path);
        assert_eq!(s, AppSettings::default());
    }

    #[test]
    fn load_returns_default_on_parse_error() {
        let dir = fresh_temp_dir("malformed");
        let path = dir.join("settings.toml");
        std::fs::write(&path, b"this is = not [valid toml @@@").expect("seed file");

        let s = load_from(&path);
        assert_eq!(s, AppSettings::default());
    }

    #[test]
    fn save_to_is_atomic_no_tmp_left_behind() {
        let dir = fresh_temp_dir("atomic");
        let path = dir.join("settings.toml");

        save_to(&path, &AppSettings::default()).expect("save_to should succeed");

        let tmp = tmp_sibling(&path);
        assert!(
            !tmp.exists(),
            "temp file {} should be gone after rename",
            tmp.display()
        );
        assert!(path.exists());
    }

    #[test]
    fn settings_path_ends_with_filename() {
        // settings_path() depends on a usable config dir on this platform,
        // which is true on macOS/Linux/Windows in CI/dev. Skip silently
        // otherwise so we don't fail on exotic runners.
        let Ok(p) = settings_path() else {
            return;
        };
        assert_eq!(
            p.file_name().and_then(|s| s.to_str()),
            Some("settings.toml")
        );
    }
}
