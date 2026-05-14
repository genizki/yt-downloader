//! Spawns a yt-dlp child process and streams [`ProgressEvent`]s to the UI.
//!
//! The public entry point is [`spawn_and_track`]. It starts the yt-dlp binary,
//! reads its stdout line-by-line, parses the JSON progress lines emitted via
//! the `--progress-template` flag set by `command_builder`, and sends
//! [`ProgressEvent`]s over the supplied `mpsc` channel.
//!
//! Stdout lines look like:
//! ```text
//! download:{"d":1234,"t":5678,"s":"downloading","p":"0.5"}
//! ```
//! The `download:` prefix is stripped before JSON parsing.

#![allow(dead_code)]

use std::process::Stdio;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::api::types::VideoId;
use crate::download::progress::{DownloadPhase, ProgressEvent};

// ---------------------------------------------------------------------------
// JSON shape
// ---------------------------------------------------------------------------

/// Intermediate struct matching the JSON emitted by yt-dlp's progress template.
#[derive(Debug, Deserialize)]
struct ProgressJson {
    /// Downloaded bytes.
    d: u64,
    /// Total bytes.
    t: u64,
    /// Status string: "downloading", "finished", etc.
    s: String,
    /// Elapsed time (unused for progress calculation; kept for completeness).
    #[allow(dead_code)]
    p: String,
}

// ---------------------------------------------------------------------------
// Line parser (pure function — easily unit-tested)
// ---------------------------------------------------------------------------

/// Parse a single stdout line from yt-dlp into a [`DownloadPhase`], or `None`
/// if the line should be ignored.
///
/// yt-dlp's `--progress-template` consumes the `<TYPE>:` prefix as a filter,
/// so the actual emitted line is the bare JSON body — but older / patched
/// versions may still echo the literal prefix. Accept both shapes; reject
/// anything that does not look like a JSON object.
pub fn parse_progress_line(line: &str) -> Option<DownloadPhase> {
    let json_str = line
        .strip_prefix("download:")
        .unwrap_or(line)
        .trim_start();
    if !json_str.starts_with('{') {
        return None;
    }
    let data: ProgressJson = serde_json::from_str(json_str).ok()?;

    match data.s.as_str() {
        "downloading" => {
            if data.t == 0 {
                // Guard against division by zero; skip the event.
                return None;
            }
            let ratio = (data.d as f32 / data.t as f32).clamp(0.0, 1.0);
            Some(DownloadPhase::Downloading(ratio))
        }
        "finished" => Some(DownloadPhase::PostProcessing),
        // Unknown / future statuses are silently skipped.
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Public async entry point
// ---------------------------------------------------------------------------

/// Spawn a yt-dlp child process and stream [`ProgressEvent`]s to `tx`.
///
/// # Arguments
/// - `video_id` — identifies the job in every event sent to the UI.
/// - `args` — the full argument vector as produced by
///   [`crate::download::command_builder::build`].
/// - `tx` — the mpsc sender; send errors (receiver dropped) are silently
///   ignored.
///
/// # Return value
/// Always returns `Ok(())`. Errors are communicated exclusively through the
/// channel: the final event is either [`DownloadPhase::Done`] or
/// [`DownloadPhase::Failed`].
pub async fn spawn_and_track(
    video_id: VideoId,
    args: Vec<std::ffi::OsString>,
    tx: tokio::sync::mpsc::Sender<ProgressEvent>,
) -> anyhow::Result<()> {
    // Resolve the bundled yt-dlp binary path.
    let binary = match crate::paths::yt_dlp_binary_path() {
        Ok(p) => p,
        Err(e) => {
            let _ = tx
                .send(ProgressEvent {
                    video_id,
                    phase: DownloadPhase::Failed(format!("failed to resolve yt-dlp path: {e}")),
                })
                .await;
            return Ok(());
        }
    };

    // Spawn the child.
    let mut child = match tokio::process::Command::new(&binary)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx
                .send(ProgressEvent {
                    video_id,
                    phase: DownloadPhase::Failed(format!("failed to spawn yt-dlp: {e}")),
                })
                .await;
            return Ok(());
        }
    };

    // Take the piped stdio handles before we move `child` into `wait()`.
    // The pipes are configured above with `.stdout(Stdio::piped())` and
    // `.stderr(Stdio::piped())`, so `take()` should normally succeed.  If a
    // sandbox or supervisor confiscated the handles, surface a `Failed`
    // event instead of panicking.
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = tx
                .send(ProgressEvent {
                    video_id,
                    phase: DownloadPhase::Failed("yt-dlp stdout pipe was not captured".to_string()),
                })
                .await;
            return Ok(());
        }
    };
    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => {
            let _ = tx
                .send(ProgressEvent {
                    video_id,
                    phase: DownloadPhase::Failed("yt-dlp stderr pipe was not captured".to_string()),
                })
                .await;
            return Ok(());
        }
    };

    // Read stdout and stderr concurrently.
    // Stderr is collected into a small ring buffer (last 20 lines).
    let tx_clone = tx.clone();
    let vid_clone = video_id.clone();

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(phase) = parse_progress_line(&line) {
                let _ = tx_clone
                    .send(ProgressEvent {
                        video_id: vid_clone.clone(),
                        phase,
                    })
                    .await;
            }
        }
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        // Ring buffer: keep only the last 20 stderr lines.
        let mut ring: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        while let Ok(Some(line)) = reader.next_line().await {
            if ring.len() == 20 {
                ring.pop_front();
            }
            ring.push_back(line);
        }
        ring
    });

    // Wait for both I/O tasks and the child process to finish.
    let (_, stderr_result, status) = tokio::join!(stdout_task, stderr_task, child.wait());

    // Determine last stderr line for failure messages.
    let last_stderr = stderr_result
        .ok()
        .and_then(|ring| ring.into_iter().last())
        .unwrap_or_default();

    // Determine exit status and emit final event.
    let phase = match status {
        Ok(s) if s.success() => DownloadPhase::Done,
        Ok(s) => {
            // s.code() is None when the process was killed by a signal (e.g. kill -9).
            let msg = match s.code() {
                None => "process terminated unexpectedly".to_string(),
                Some(code) => {
                    if last_stderr.is_empty() {
                        format!("yt-dlp exited with code {code}")
                    } else {
                        last_stderr
                    }
                }
            };
            DownloadPhase::Failed(msg)
        }
        Err(e) => DownloadPhase::Failed(format!("failed to wait for yt-dlp: {e}")),
    };

    let _ = tx.send(ProgressEvent { video_id, phase }).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloading_half_progress() {
        let result =
            parse_progress_line(r#"download:{"d":500,"t":1000,"s":"downloading","p":"1.0"}"#);
        assert_eq!(result, Some(DownloadPhase::Downloading(0.5)));
    }

    #[test]
    fn downloading_zero_total_returns_none() {
        let result = parse_progress_line(r#"download:{"d":0,"t":0,"s":"downloading","p":"0"}"#);
        assert_eq!(result, None, "expected None when total bytes is 0");
    }

    #[test]
    fn finished_status_maps_to_post_processing() {
        let result = parse_progress_line(r#"download:{"d":0,"t":1000,"s":"finished","p":"5.2"}"#);
        assert_eq!(result, Some(DownloadPhase::PostProcessing));
    }

    #[test]
    fn bare_json_line_is_accepted() {
        // yt-dlp normally emits the JSON without the `download:` prefix
        // (the prefix is consumed as the template TYPE filter), so the parser
        // must accept both shapes.
        assert_eq!(
            parse_progress_line(r#"{"d":500,"t":1000,"s":"downloading","p":"1.0"}"#),
            Some(DownloadPhase::Downloading(0.5))
        );
    }

    #[test]
    fn non_json_line_returns_none() {
        assert_eq!(parse_progress_line("some random output from yt-dlp"), None);
        assert_eq!(parse_progress_line(""), None);
    }

    #[test]
    fn unknown_status_returns_none() {
        let result = parse_progress_line(r#"download:{"d":200,"t":1000,"s":"error","p":"2.1"}"#);
        assert_eq!(result, None);
    }

    #[test]
    fn progress_clamped_to_one() {
        // d > t would be unusual but should not produce a value > 1.0.
        let result =
            parse_progress_line(r#"download:{"d":2000,"t":1000,"s":"downloading","p":"3.0"}"#);
        assert_eq!(result, Some(DownloadPhase::Downloading(1.0)));
    }

    #[test]
    fn invalid_json_returns_none() {
        assert_eq!(parse_progress_line("download:not-json"), None);
    }
}
