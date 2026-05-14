//! The built yt-dlp command and helpers to consume it.
//!
//! [`Command`] is the final output of
//! [`super::builder::CommandBuilder::build`]. It bundles the argument vector
//! with diagnostic info: which features require an external `ffmpeg` binary
//! (rule 5 of the task spec) and any non-fatal warnings (e.g. `simulate`
//! mode, rule 2).

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Final built command, ready to be handed to `std::process::Command`.
#[derive(Debug, Clone)]
pub struct Command {
    pub(crate) args: Vec<OsString>,
    pub(crate) requires_ffmpeg: bool,
    pub(crate) ffmpeg_reasons: Vec<&'static str>,
    pub(crate) warnings: Vec<String>,
    pub(crate) yt_dlp_binary: Option<PathBuf>,
}

impl Command {
    /// Borrow the assembled `yt-dlp` argument vector.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// Take ownership of the argument vector (avoids a `.to_vec()` clone at
    /// call sites that hand the args to a downstream spawner).
    pub fn into_args(self) -> Vec<OsString> {
        self.args
    }

    /// Whether the command needs the `ffmpeg` binary at runtime. Set when any
    /// option requires merging, extraction, or chapter post-processing.
    pub fn requires_ffmpeg(&self) -> bool {
        self.requires_ffmpeg
    }

    /// Static labels describing why ffmpeg is required. Useful for diagnostic
    /// UIs that want to explain a missing-binary error.
    pub fn ffmpeg_reasons(&self) -> &[&'static str] {
        &self.ffmpeg_reasons
    }

    /// Non-fatal warnings emitted during build (e.g. `simulate=true`).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Render the command as a single-line shell string for debug/log output.
    ///
    /// **Not** intended for re-execution: only inputs the builder produced
    /// itself are present, all of which come from internal enums and known
    /// paths (no untrusted user free-text). Quoting handles whitespace and a
    /// small set of shell metacharacters but is deliberately simple.
    pub fn to_shell_string(&self) -> String {
        let bin = self
            .yt_dlp_binary
            .as_deref()
            .map(Path::to_string_lossy)
            .unwrap_or("yt-dlp".into())
            .into_owned();
        let mut out = bin;
        for a in &self.args {
            out.push(' ');
            out.push_str(&shell_quote(a));
        }
        out
    }

    /// Set the `yt-dlp` binary used when the command is materialised via
    /// [`Self::into_std`] or rendered via [`Self::to_shell_string`].
    pub fn with_binary(mut self, p: impl Into<PathBuf>) -> Self {
        self.yt_dlp_binary = Some(p.into());
        self
    }

    /// Convert into a [`std::process::Command`]. Uses `self.yt_dlp_binary` if
    /// set, otherwise falls back to a `yt-dlp` lookup via `PATH`.
    pub fn into_std(self) -> std::process::Command {
        let bin = self
            .yt_dlp_binary
            .clone()
            .unwrap_or_else(|| PathBuf::from("yt-dlp"));
        let mut cmd = std::process::Command::new(bin);
        cmd.args(&self.args);
        cmd
    }
}

impl From<Command> for std::process::Command {
    fn from(c: Command) -> Self {
        c.into_std()
    }
}

fn shell_quote(s: &OsStr) -> String {
    let s = s.to_string_lossy();
    let needs_quote = s.is_empty()
        || s.chars()
            .any(|c| c.is_whitespace() || " *~|[]{}()<>$#&;'\"\\`".contains(c));
    if !needs_quote {
        return s.into_owned();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str(r"'\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}
