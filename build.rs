//! Build script: downloads the yt-dlp binaries for all three platforms into
//! `bin/yt-dlp/<os>/` so the GUI ships with a known-good binary per platform.
//!
//! Behavior:
//! - For each target (windows, macos, linux): download the upstream binary
//!   from `https://github.com/yt-dlp/yt-dlp/releases/latest/download/<file>`
//!   and verify its SHA256 against the upstream `SHA2-256SUMS` file.
//! - Idempotent: if the destination file already exists, the download is
//!   skipped. We deliberately do NOT re-hash existing files on every build —
//!   the SHA256 verification happens once, right after the download. This
//!   keeps incremental builds fast. To force a re-download, delete the file
//!   (or run `cargo clean`).
//! - Opt-out: setting the env var `YT_DLP_GUI_SKIP_BIN_DOWNLOAD=1` skips the
//!   downloads entirely (useful offline / when iterating fast).
//! - Unix permissions: the macOS and Linux binaries are chmod 0o755 after
//!   download.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};

const RELEASE_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const SHA_FILE: &str = "SHA2-256SUMS";
const HTTP_TIMEOUT_SECS: u64 = 60;

/// One bundled-binary target: which upstream file to fetch and where it ends up.
struct Target {
    /// Subdirectory under `bin/yt-dlp/` (also used in `paths.rs` for lookup).
    os_dir: &'static str,
    /// Upstream filename in the GitHub release.
    upstream_name: &'static str,
    /// Local filename inside `bin/yt-dlp/<os_dir>/`.
    local_name: &'static str,
    /// Whether to chmod 0o755 after download (Unix binaries only).
    make_executable: bool,
}

const TARGETS: &[Target] = &[
    Target {
        os_dir: "windows",
        upstream_name: "yt-dlp.exe",
        local_name: "yt-dlp.exe",
        make_executable: false,
    },
    Target {
        os_dir: "macos",
        upstream_name: "yt-dlp_macos",
        local_name: "yt-dlp",
        make_executable: true,
    },
    Target {
        os_dir: "linux",
        upstream_name: "yt-dlp_linux",
        local_name: "yt-dlp",
        make_executable: true,
    },
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=YT_DLP_GUI_SKIP_BIN_DOWNLOAD");

    if std::env::var("YT_DLP_GUI_SKIP_BIN_DOWNLOAD").as_deref() == Ok("1") {
        println!(
            "cargo:warning=YT_DLP_GUI_SKIP_BIN_DOWNLOAD=1 set — skipping yt-dlp binary download. \
             The GUI will fail to spawn yt-dlp at runtime unless binaries are placed manually under bin/yt-dlp/<os>/."
        );
        return;
    }

    if let Err(e) = run() {
        // Don't fail the build — print a warning so devs can still iterate
        // (e.g. offline). Runtime will simply not find the binary.
        println!(
            "cargo:warning=yt-dlp binary download failed: {e:#}. \
             Set YT_DLP_GUI_SKIP_BIN_DOWNLOAD=1 to silence this, or place binaries manually."
        );
    }
}

fn run() -> Result<()> {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR not set")?);
    let bin_root = manifest_dir.join("bin").join("yt-dlp");

    // Skip everything if all targets are already present — saves the SHA-file
    // round trip on hot incremental builds.
    if TARGETS
        .iter()
        .all(|t| bin_root.join(t.os_dir).join(t.local_name).exists())
    {
        return Ok(());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .user_agent("yt-dlp-gui-build/0.1")
        .build()
        .context("failed to build reqwest client")?;

    let sha_map = fetch_sha_map(&client).context("failed to fetch SHA2-256SUMS")?;

    for target in TARGETS {
        let dir = bin_root.join(target.os_dir);
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create dir {}", dir.display()))?;
        let dest = dir.join(target.local_name);

        if dest.exists() {
            // Idempotent path. We trust an already-present binary; rehashing
            // every build slows things down for negligible safety win.
            continue;
        }

        let expected = sha_map.get(target.upstream_name).ok_or_else(|| {
            anyhow!(
                "no SHA256 found for {} in upstream SHA2-256SUMS",
                target.upstream_name
            )
        })?;

        download_and_verify(&client, target, &dest, expected)?;

        if target.make_executable {
            set_executable(&dest)?;
        }

        println!(
            "cargo:warning=downloaded {} -> {} (SHA256 ok)",
            target.upstream_name,
            dest.display()
        );
    }

    Ok(())
}

fn fetch_sha_map(client: &reqwest::blocking::Client) -> Result<HashMap<String, String>> {
    let url = format!("{RELEASE_BASE}/{SHA_FILE}");
    let body = client
        .get(&url)
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("non-2xx for {url}"))?
        .text()
        .context("read SHA file body")?;

    let mut map = HashMap::new();
    // Format per line: "<hex-sha256>  <filename>" (two spaces per coreutils convention).
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(hash), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        map.insert(name.to_string(), hash.to_lowercase());
    }
    Ok(map)
}

fn download_and_verify(
    client: &reqwest::blocking::Client,
    target: &Target,
    dest: &Path,
    expected_hex: &str,
) -> Result<()> {
    let url = format!("{}/{}", RELEASE_BASE, target.upstream_name);
    let bytes = client
        .get(&url)
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("non-2xx for {url}"))?
        .bytes()
        .with_context(|| format!("read body of {url}"))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = hex::encode(hasher.finalize());
    if !got.eq_ignore_ascii_case(expected_hex) {
        return Err(anyhow!(
            "SHA256 mismatch for {}: expected {}, got {}",
            target.upstream_name,
            expected_hex,
            got
        ));
    }

    // Atomic-ish write: write to tmp, then rename.
    let tmp = dest.with_extension("part");
    {
        let mut f = fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        f.write_all(&bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, dest)
        .with_context(|| format!("rename {} -> {}", tmp.display(), dest.display()))?;
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).with_context(|| format!("chmod {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    // No-op on non-Unix hosts: Windows uses .exe extension semantics.
    Ok(())
}
