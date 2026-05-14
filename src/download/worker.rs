//! Per-worker isolated download queue.
//!
//! Each worker owns its own [`VecDeque<DownloadJob>`] and processes jobs
//! sequentially with no shared mutable state. Communication back to the UI
//! thread happens exclusively through a `tokio::sync::mpsc` channel via
//! [`ProgressEvent`] messages.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::api::types::VideoId;
use crate::download::progress::{DownloadPhase, ProgressEvent};
use crate::settings::AppSettings;

// ---------------------------------------------------------------------------
// Job type
// ---------------------------------------------------------------------------

/// A single unit of work for a download worker.
///
/// The settings are captured as a snapshot at dispatch time so changes made
/// by the user while a job is running do not affect that job.
#[derive(Clone, Debug)]
pub struct DownloadJob {
    pub video_id: VideoId,
    /// Settings snapshot captured when the job was dispatched.
    pub settings: AppSettings,
    /// Temporary directory in which yt-dlp should write the intermediate file.
    /// The worker moves the finished file to `settings.download_path` after
    /// yt-dlp exits successfully.
    pub temp_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// Worker entry point
// ---------------------------------------------------------------------------

/// Drive a queue of [`DownloadJob`]s to completion, one at a time.
///
/// For each job the worker:
/// 1. Sends a [`DownloadPhase::Queued`] event.
/// 2. Builds the yt-dlp argument vector via [`crate::download::command_builder::build`].
/// 3. Delegates to [`crate::download::yt_dlp::spawn_and_track`], which
///    streams `Downloading` / `PostProcessing` / `Done` / `Failed` events.
/// 4. On success, locates the output file in `temp_dir` and moves it to
///    `settings.download_path`, sending a [`DownloadPhase::Moving`] event
///    before the rename and a final [`DownloadPhase::Done`] (or
///    [`DownloadPhase::Failed`]) afterwards.
///
/// Errors from `tx.send()` are silently ignored — the UI may drop the
/// receiver at any time and that is not a fatal condition for the worker.
pub async fn run_worker(
    mut queue: VecDeque<DownloadJob>,
    tx: mpsc::Sender<ProgressEvent>,
    _slot: crate::download::manager::WorkerSlot,
) {
    while let Some(job) = queue.pop_front() {
        // Step 1: announce that this job has been picked up.
        let _ = tx
            .send(ProgressEvent {
                video_id: job.video_id.clone(),
                phase: DownloadPhase::Queued,
            })
            .await;

        // Step 2: build the yt-dlp argument vector.
        //
        // Resolve ffmpeg once per job. yt-dlp needs it to merge `bv*+ba` and
        // for `-x` audio extraction; without `--ffmpeg-location` it falls back
        // to PATH only and silently fails when ffmpeg is missing.
        let ffmpeg = crate::paths::ffmpeg_binary_path();
        if ffmpeg.is_none() {
            eprintln!(
                "warning: ffmpeg not found (checked <exe-dir>/bin and PATH). \
                 yt-dlp will fail for merged-video and audio-extraction downloads."
            );
        }
        let cmd = crate::download::command_builder::CommandBuilder::from_settings(
            &job.settings,
            job.video_id.as_str(),
            &job.temp_dir,
        )
        .ffmpeg_location_opt(ffmpeg.as_deref())
        .build()
        .expect("settings-derived command must always validate");
        let args = cmd.into_args();

        // Step 3: spawn yt-dlp and stream progress events.
        // `spawn_and_track` always returns `Ok(())`; errors are reported via
        // the channel as `DownloadPhase::Failed`.
        let _ =
            crate::download::yt_dlp::spawn_and_track(job.video_id.clone(), args, tx.clone()).await;

        // Step 4: check the last event sent by spawn_and_track to decide
        // whether to attempt the move.  Since we cannot inspect the channel
        // retroactively we probe the temp dir: if a file matching the video id
        // exists there, spawn_and_track reported Done; otherwise we skip the
        // move (a Failed event was already sent).
        let maybe_src = find_output_file(&job.temp_dir, job.video_id.as_str()).await;

        if let Some(src) = maybe_src {
            // Announce the move phase.
            let _ = tx
                .send(ProgressEvent {
                    video_id: job.video_id.clone(),
                    phase: DownloadPhase::Moving,
                })
                .await;

            let phase = match src.file_name() {
                None => DownloadPhase::Failed(format!(
                    "output path has no file name: {}",
                    src.display()
                )),
                Some(name) => {
                    let dst = job.settings.download_path.join(name);
                    match move_file(&src, &dst).await {
                        Ok(()) => DownloadPhase::Done,
                        Err(e) => DownloadPhase::Failed(format!("move failed: {e}")),
                    }
                }
            };

            let _ = tx
                .send(ProgressEvent {
                    video_id: job.video_id.clone(),
                    phase,
                })
                .await;
        }
        // If `maybe_src` is None, yt-dlp already sent a Failed event — nothing
        // more to do for this job.
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Search `dir` for the first file whose stem contains `video_id` (any
/// extension).  Returns `None` if the directory cannot be read or no matching
/// file exists.
async fn find_output_file(dir: &std::path::Path, video_id: &str) -> Option<PathBuf> {
    let mut read_dir = tokio::fs::read_dir(dir).await.ok()?;
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.contains(video_id) {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Move `src` to `dst`.
///
/// Tries a cheap `rename` first; on cross-device failure falls back to a
/// copy + delete.
async fn move_file(src: &std::path::Path, dst: &std::path::Path) -> anyhow::Result<()> {
    match tokio::fs::rename(src, dst).await {
        Ok(()) => Ok(()),
        Err(_) => {
            // Cross-device move: copy then remove the source.
            tokio::fs::copy(src, dst).await?;
            tokio::fs::remove_file(src).await?;
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that the very first event emitted for a queued job is
    /// `DownloadPhase::Queued`, even when yt-dlp is not installed.
    ///
    /// `spawn_and_track` handles a missing binary gracefully (it sends a
    /// `Failed` event instead of panicking), so this test can run in any CI
    /// environment.
    #[tokio::test]
    async fn worker_sends_queued_first() {
        use std::sync::atomic::AtomicUsize;
        use std::sync::Arc;

        let (tx, mut rx) = mpsc::channel(10);
        let mut queue = VecDeque::new();
        queue.push_back(DownloadJob {
            video_id: VideoId::new("test_id"),
            settings: AppSettings::default(),
            temp_dir: std::env::temp_dir().join("yt-dlp-gui-test"),
        });

        let counter = Arc::new(AtomicUsize::new(1));
        let slot = crate::download::manager::WorkerSlot::new_for_test(Arc::clone(&counter));
        tokio::spawn(run_worker(queue, tx, slot));

        let first = rx.recv().await.expect("first event");
        assert_eq!(first.video_id.as_str(), "test_id");
        assert!(
            matches!(first.phase, DownloadPhase::Queued),
            "expected Queued, got {:?}",
            first.phase
        );
    }

    /// Sanity-check: a path with no file name (the filesystem root) does not
    /// panic; the worker reports a `Failed` phase and continues.
    #[test]
    fn root_path_has_no_file_name() {
        // The behaviour we depend on: `Path::file_name` returns `None` for `/`
        // (and `\\` on Windows). We do not exercise the worker end-to-end here
        // because spawning `yt-dlp` is out of scope for unit tests; this test
        // documents the invariant the patch relies on.
        use std::path::Path;
        assert!(Path::new("/").file_name().is_none());
    }
}
