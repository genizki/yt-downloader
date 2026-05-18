//! Download-job dispatcher.
//!
//! Concentrates the three dispatch paths (single, batch from selection,
//! auto-download from playlist) behind one interface. Builds [`DownloadJob`]s,
//! applies the in-flight guard, resolves the temp directory, and emits debug
//! logs in one place.
//!
//! See `CONTEXT.md` → "DownloadJob", "Worker".

#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::api::types::{VideoId, YouTubeVideo};
use crate::download::progress::{DownloadPhase, ProgressEvent};
use crate::download::worker::{run_worker, DownloadJob};
use crate::settings::AppSettings;

/// RAII guard that decrements the shared worker counter when dropped.
///
/// Created when a worker task is spawned and moved into `run_worker`. When the
/// worker finishes — normally, by panic, or by being cancelled — the `Drop`
/// impl releases the slot.
struct WorkerSlot {
    counter: Arc<AtomicUsize>,
}

impl WorkerSlot {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self { counter }
    }
}

impl Drop for WorkerSlot {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Maximum number of concurrent workers (yt-dlp community soft limit).
/// Higher values trigger YouTube 429s.
pub const MAX_CONCURRENT: usize = 5;

/// Distribute `jobs` evenly across `n_workers` buckets using round-robin.
///
/// Guarantees: no bucket gets more than 1 extra job compared to others
/// (bucket 0 gets ceil, rest get floor — not "first bucket gets all excess").
/// Example: 7 jobs, 5 workers -> buckets of sizes [2, 2, 1, 1, 1].
fn round_robin_split(jobs: Vec<DownloadJob>, n_workers: usize) -> Vec<VecDeque<DownloadJob>> {
    let mut buckets: Vec<VecDeque<DownloadJob>> = (0..n_workers).map(|_| VecDeque::new()).collect();
    for (i, job) in jobs.into_iter().enumerate() {
        buckets[i % n_workers].push_back(job);
    }
    buckets
}

/// Per-video result of a dispatch call.
///
/// `Queued` means a worker accepted the job; `AlreadyInFlight` means the
/// in-flight guard rejected it (the video already has a `Queued`,
/// `Downloading`, or `PostProcessing` phase); `NoSlot` means
/// [`MAX_CONCURRENT`] workers are already running and the dispatcher dropped
/// the job — the caller should retry on a later tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome {
    Queued,
    AlreadyInFlight,
    NoSlot,
}

/// Single point of entry for queuing downloads.
pub struct DownloadDispatcher {
    /// Shared sender — cloned for each worker spawn.
    tx: mpsc::Sender<ProgressEvent>,
    /// Number of currently running workers, shared with each worker via a
    /// `WorkerSlot` guard so the count decrements on completion.
    active_workers: Arc<AtomicUsize>,
    /// Token handed to every spawned worker. `cancel_all` cancels it and
    /// installs a fresh token so subsequent dispatches are unaffected.
    cancel: CancellationToken,
}

impl DownloadDispatcher {
    pub fn new(progress_tx: mpsc::Sender<ProgressEvent>) -> Self {
        Self {
            tx: progress_tx,
            active_workers: Arc::new(AtomicUsize::new(0)),
            cancel: CancellationToken::new(),
        }
    }

    /// Snapshot of currently running workers.
    pub fn active_count(&self) -> usize {
        self.active_workers.load(Ordering::Acquire)
    }

    /// Cancel every in-flight worker and install a fresh token. Workers
    /// observe the cancellation, kill their child process, and drain their
    /// queue. New dispatches after this call use the fresh token.
    pub fn cancel_all(&mut self) {
        self.cancel.cancel();
        self.cancel = CancellationToken::new();
    }

    /// Dispatch a single user-initiated download. Returns `AlreadyInFlight`
    /// when the video is already being processed; `NoSlot` when the worker
    /// cap is full.
    pub fn dispatch_single(
        &mut self,
        video_id: VideoId,
        settings: &AppSettings,
        phases: &HashMap<VideoId, DownloadPhase>,
    ) -> DispatchOutcome {
        if is_in_flight(phases.get(&video_id)) {
            tracing::debug!(id = %video_id.0, "dispatch_single: already in-flight");
            return DispatchOutcome::AlreadyInFlight;
        }
        if self.active_count() >= MAX_CONCURRENT {
            tracing::warn!(id = %video_id.0, "dispatch_single: MAX_CONCURRENT reached");
            return DispatchOutcome::NoSlot;
        }
        if crate::debug::enabled() {
            crate::debug::log_ytdlp_command(settings);
        }
        let job = DownloadJob {
            video_id,
            settings: settings.clone(),
            temp_dir: resolve_temp_dir(),
        };
        self.dispatch_single_job(job);
        DispatchOutcome::Queued
    }

    /// Dispatch a batch of user-initiated downloads (e.g. from the selection
    /// set). Per-video outcomes are returned in the input order: in-flight
    /// videos are skipped, and any job beyond the available worker slots is
    /// reported as `NoSlot` (the dispatcher drops them; the caller can retry).
    pub fn dispatch_many(
        &mut self,
        video_ids: Vec<VideoId>,
        settings: &AppSettings,
        phases: &HashMap<VideoId, DownloadPhase>,
    ) -> Vec<(VideoId, DispatchOutcome)> {
        let mut outcomes: Vec<(VideoId, DispatchOutcome)> = Vec::with_capacity(video_ids.len());
        let mut accepted: Vec<VideoId> = Vec::new();

        for id in video_ids {
            if is_in_flight(phases.get(&id)) {
                outcomes.push((id, DispatchOutcome::AlreadyInFlight));
            } else {
                accepted.push(id);
            }
        }

        if accepted.is_empty() {
            return outcomes;
        }

        if crate::debug::enabled() {
            crate::debug::log_ytdlp_command(settings);
        }

        let available = MAX_CONCURRENT.saturating_sub(self.active_count());
        let queued_count = accepted.len().min(available);
        let temp_dir = resolve_temp_dir();
        let jobs: Vec<DownloadJob> = accepted
            .iter()
            .take(queued_count)
            .cloned()
            .map(|video_id| DownloadJob {
                video_id,
                settings: settings.clone(),
                temp_dir: temp_dir.clone(),
            })
            .collect();
        tracing::debug!(count = jobs.len(), "dispatch_many: queuing batch");
        self.dispatch_batch(jobs);

        for (i, id) in accepted.into_iter().enumerate() {
            outcomes.push((
                id,
                if i < queued_count {
                    DispatchOutcome::Queued
                } else {
                    DispatchOutcome::NoSlot
                },
            ));
        }
        outcomes
    }

    /// Auto-dispatch a freshly resolved playlist. No in-flight guard is
    /// applied: the videos came from a fresh API response, so no phase
    /// entries can exist for them yet. The dispatcher still caps at
    /// [`MAX_CONCURRENT`] internally.
    pub fn dispatch_auto(&mut self, videos: Vec<YouTubeVideo>, settings: &AppSettings) {
        let temp_dir = resolve_temp_dir();
        let jobs: Vec<DownloadJob> = videos
            .into_iter()
            .map(|v| DownloadJob {
                video_id: v.id,
                settings: settings.clone(),
                temp_dir: temp_dir.clone(),
            })
            .collect();
        tracing::debug!(count = jobs.len(), "dispatch_auto: playlist batch");
        self.dispatch_batch(jobs);
    }

    /// Dispatch a batch of jobs across workers (round-robin).
    ///
    /// Caps the number of spawned workers at [`MAX_CONCURRENT`] total.
    fn dispatch_batch(&mut self, jobs: Vec<DownloadJob>) {
        let available = MAX_CONCURRENT.saturating_sub(self.active_count());
        let n = jobs.len().min(available);
        if n == 0 {
            return;
        }
        let buckets = round_robin_split(jobs, n);
        for bucket in buckets {
            if bucket.is_empty() {
                continue;
            }
            let tx = self.tx.clone();
            self.active_workers.fetch_add(1, Ordering::AcqRel);
            let slot = WorkerSlot::new(Arc::clone(&self.active_workers));
            let cancel = self.cancel.clone();
            tokio::spawn(run_worker(bucket, tx, slot, cancel));
        }
    }

    /// Dispatch a single job.
    ///
    /// If fewer than [`MAX_CONCURRENT`] workers are active, spawns a new worker
    /// with a 1-job queue and increments the active count.
    ///
    /// If the cap is already reached, logs a warning and drops the job. The
    /// caller is expected to retry on the next frame.
    fn dispatch_single_job(&mut self, job: DownloadJob) {
        if self.active_count() < MAX_CONCURRENT {
            let mut queue = VecDeque::with_capacity(1);
            queue.push_back(job);
            let tx = self.tx.clone();
            self.active_workers.fetch_add(1, Ordering::AcqRel);
            let slot = WorkerSlot::new(Arc::clone(&self.active_workers));
            let cancel = self.cancel.clone();
            tokio::spawn(run_worker(queue, tx, slot, cancel));
        } else {
            tracing::warn!(
                "dispatch_single: MAX_CONCURRENT ({}) reached, dropping job — caller should retry",
                MAX_CONCURRENT
            );
        }
    }
}

/// Pure predicate: is the given phase one of the active states that should
/// block a new dispatch for the same video?
fn is_in_flight(phase: Option<&DownloadPhase>) -> bool {
    matches!(
        phase,
        Some(DownloadPhase::Queued)
            | Some(DownloadPhase::Downloading { .. })
            | Some(DownloadPhase::PostProcessing)
    )
}

/// Resolve the temp directory with the same fallback the legacy code used:
/// the platform-specific app temp dir, or the OS temp dir + `yt-dlp-gui`.
fn resolve_temp_dir() -> PathBuf {
    crate::paths::temp_dir().unwrap_or_else(|_| std::env::temp_dir().join("yt-dlp-gui"))
}

// ---------------------------------------------------------------------------
// Unit tests — pure predicate only; dispatch_* tests require a running tokio
// runtime and live in integration tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::VideoId;
    use crate::settings::AppSettings;

    fn make_jobs(n: usize) -> Vec<DownloadJob> {
        (0..n)
            .map(|i| DownloadJob {
                video_id: VideoId::new(format!("id{i}")),
                settings: AppSettings::default(),
                temp_dir: std::env::temp_dir(),
            })
            .collect()
    }

    #[test]
    fn queued_is_in_flight() {
        assert!(is_in_flight(Some(&DownloadPhase::Queued)));
    }

    #[test]
    fn downloading_is_in_flight() {
        assert!(is_in_flight(Some(&DownloadPhase::Downloading {
            progress: 0.5
        })));
    }

    #[test]
    fn post_processing_is_in_flight() {
        assert!(is_in_flight(Some(&DownloadPhase::PostProcessing)));
    }

    #[test]
    fn moving_is_not_in_flight() {
        // Moving comes after the worker has finished its yt-dlp work; a new
        // dispatch at this point would conflict on the output file, but the
        // legacy code did not guard against it. Preserve that behaviour
        // until we have a real reason to change it.
        assert!(!is_in_flight(Some(&DownloadPhase::Moving)));
    }

    #[test]
    fn done_is_not_in_flight() {
        assert!(!is_in_flight(Some(&DownloadPhase::Done)));
    }

    #[test]
    fn failed_is_not_in_flight() {
        // A user may want to retry a failed download — must not be guarded.
        assert!(!is_in_flight(Some(&DownloadPhase::Failed {
            error: "err".into()
        })));
    }

    #[test]
    fn no_phase_is_not_in_flight() {
        assert!(!is_in_flight(None));
    }

    #[test]
    fn round_robin_split_distributes_evenly() {
        // 7 jobs, 5 workers -> sizes [2, 2, 1, 1, 1]
        let jobs = make_jobs(7);
        let buckets = round_robin_split(jobs, 5);
        assert_eq!(buckets.len(), 5);
        let sizes: Vec<usize> = buckets.iter().map(|b| b.len()).collect();
        assert_eq!(sizes, vec![2, 2, 1, 1, 1]);
    }

    #[test]
    fn round_robin_split_single_worker() {
        let jobs = make_jobs(3);
        let buckets = round_robin_split(jobs, 1);
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].len(), 3);
    }

    #[test]
    fn round_robin_split_more_workers_than_jobs() {
        let jobs = make_jobs(2);
        let buckets = round_robin_split(jobs, 5);
        // Only 2 non-empty buckets needed; but we still return 5 buckets
        // (empty ones are fine — dispatch_batch skips empty buckets)
        assert_eq!(buckets.len(), 5);
        let total: usize = buckets.iter().map(|b| b.len()).sum();
        assert_eq!(total, 2);
    }

    #[test]
    fn round_robin_split_property_total_preserved() {
        for n_jobs in 0..=50usize {
            for n_workers in 1..=5usize {
                let jobs = make_jobs(n_jobs);
                let buckets = round_robin_split(jobs, n_workers);
                let total: usize = buckets.iter().map(|b| b.len()).sum();
                assert_eq!(total, n_jobs, "n_jobs={n_jobs} n_workers={n_workers}");
                // Max size diff between buckets ≤ 1
                let max = buckets.iter().map(|b| b.len()).max().unwrap_or(0);
                let min = buckets.iter().map(|b| b.len()).min().unwrap_or(0);
                assert!(
                    max - min <= 1,
                    "uneven split: n_jobs={n_jobs} n_workers={n_workers} max={max} min={min}"
                );
            }
        }
    }

    #[test]
    fn round_robin_split_empty_jobs() {
        let buckets = round_robin_split(vec![], 3);
        assert_eq!(buckets.len(), 3);
        assert!(buckets.iter().all(|b| b.is_empty()));
    }

    #[test]
    fn round_robin_split_preserves_order() {
        // First bucket should always get jobs 0, n_workers, 2*n_workers, ...
        let jobs = make_jobs(6);
        let ids: Vec<String> = jobs
            .iter()
            .map(|j| j.video_id.as_str().to_string())
            .collect();
        let buckets = round_robin_split(jobs, 3);
        assert_eq!(buckets[0][0].video_id.as_str(), ids[0]);
        assert_eq!(buckets[0][1].video_id.as_str(), ids[3]);
        assert_eq!(buckets[1][0].video_id.as_str(), ids[1]);
        assert_eq!(buckets[2][0].video_id.as_str(), ids[2]);
    }
}
