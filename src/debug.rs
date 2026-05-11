//! Debug-mode helpers.
//!
//! Enable at runtime with `--debug`. When active, the tracing filter is set
//! to `debug` level and key actions log their full context to stderr.
//!
//! Usage anywhere in the codebase:
//! ```rust
//! if crate::debug::enabled() {
//!     crate::debug::log_ytdlp_command(&settings);
//! }
//! tracing::debug!("any structured message");
//! ```

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG: AtomicBool = AtomicBool::new(false);

/// Call once from `main` when `--debug` is present in argv.
pub fn enable() {
    DEBUG.store(true, Ordering::Relaxed);
}

/// Returns `true` when the process was started with `--debug`.
#[inline]
pub fn enabled() -> bool {
    DEBUG.load(Ordering::Relaxed)
}

/// Build the full yt-dlp argument list from the current settings and emit it
/// as a single readable `debug!` line.  Uses `<VIDEO_ID>` and `<TEMP>` as
/// placeholders so the preview is settings-only, no real video required.
pub fn log_ytdlp_command(settings: &crate::settings::AppSettings) {
    if !enabled() {
        return;
    }

    let args = crate::download::command_builder::build(
        settings,
        "<VIDEO_ID>",
        Path::new("<TEMP>"),
    );

    // Render each OsString; quote values that contain spaces or shell-special chars.
    let parts: Vec<String> = args
        .iter()
        .map(|a| {
            let s = a.to_string_lossy();
            if s.chars().any(|c| " *~|[]{}()<>$#&;".contains(c)) {
                format!("'{s}'")
            } else {
                s.into_owned()
            }
        })
        .collect();

    // Split into lines of at most ~80 chars for readability.
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::from("yt-dlp");
    for part in &parts {
        if current.len() + 1 + part.len() > 80 {
            current.push_str(" \\");
            lines.push(current.clone());
            current = format!("  {part}");
        } else {
            current.push(' ');
            current.push_str(part);
        }
    }
    lines.push(current);

    tracing::debug!("yt-dlp command preview:\n{}", lines.join("\n"));
}
