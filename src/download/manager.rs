//! Download manager: coordinates multiple workers via round-robin distribution.
//!
//! Key design constraint: **NO shared mutable job list**. Each worker owns its
//! isolated [`VecDeque<DownloadJob>`] and processes jobs sequentially with no
//! shared mutable state.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::download::progress::ProgressEvent;
use crate::download::worker::{run_worker, DownloadJob};

/// RAII guard that decrements the shared worker counter when dropped.
///
/// Created when a worker task is spawned and moved into `run_worker`.  When
/// the worker finishes — normally, by panic, or by being cancelled — the
/// `Drop` impl releases the slot.
pub struct WorkerSlot {
    counter: Arc<AtomicUsize>,
}

impl WorkerSlot {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self { counter }
    }

    /// Test-only constructor that lets other modules build a `WorkerSlot`
    /// without going through `dispatch_*`.  The caller is responsible for
    /// pre-incrementing `counter` to mirror what `dispatch_*` would do.
    #[cfg(test)]
    pub fn new_for_test(counter: Arc<AtomicUsize>) -> Self {
        Self::new(counter)
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

// ---------------------------------------------------------------------------
// Round-robin split
// ---------------------------------------------------------------------------

/// Distribute `jobs` evenly across `n_workers` buckets using round-robin.
///
/// Guarantees: no bucket gets more than 1 extra job compared to others
/// (bucket 0 gets ceil, rest get floor — not "first bucket gets all excess").
/// Example: 7 jobs, 5 workers → buckets of sizes [2, 2, 1, 1, 1].
pub fn round_robin_split(jobs: Vec<DownloadJob>, n_workers: usize) -> Vec<VecDeque<DownloadJob>> {
    let mut buckets: Vec<VecDeque<DownloadJob>> = (0..n_workers).map(|_| VecDeque::new()).collect();
    for (i, job) in jobs.into_iter().enumerate() {
        buckets[i % n_workers].push_back(job);
    }
    buckets
}

// ---------------------------------------------------------------------------
// DownloadManager
// ---------------------------------------------------------------------------

/// Coordinates multiple download workers.
///
/// Each worker runs as an isolated tokio task owning its own
/// [`VecDeque<DownloadJob>`]. The manager is responsible for distributing
/// jobs across workers and enforcing the [`MAX_CONCURRENT`] cap.
pub struct DownloadManager {
    /// Shared sender — cloned for each worker spawn.
    tx: mpsc::Sender<ProgressEvent>,
    /// Number of currently running workers, shared with each worker via a
    /// `WorkerSlot` guard so the count decrements on completion.
    active_workers: Arc<AtomicUsize>,
}

impl DownloadManager {
    /// Create a new [`DownloadManager`] that sends progress events via `tx`.
    pub fn new(tx: mpsc::Sender<ProgressEvent>) -> Self {
        Self {
            tx,
            active_workers: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Snapshot of the current worker count.  Useful for tests and the UI.
    pub fn active_count(&self) -> usize {
        self.active_workers.load(Ordering::Acquire)
    }

    /// Dispatch a batch of jobs across workers (round-robin).
    ///
    /// Caps the number of spawned workers at [`MAX_CONCURRENT`] total.
    pub fn dispatch_batch(&mut self, jobs: Vec<DownloadJob>) {
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
            tokio::spawn(run_worker(bucket, tx, slot));
        }
    }

    /// Dispatch a single job.
    ///
    /// If fewer than [`MAX_CONCURRENT`] workers are active, spawns a new
    /// worker with a 1-job queue and increments the active count.
    ///
    /// If the cap is already reached, logs a warning and drops the job.
    /// The caller is expected to retry on the next frame.
    pub fn dispatch_single(&mut self, job: DownloadJob) {
        if self.active_count() < MAX_CONCURRENT {
            let mut queue = VecDeque::with_capacity(1);
            queue.push_back(job);
            let tx = self.tx.clone();
            self.active_workers.fetch_add(1, Ordering::AcqRel);
            let slot = WorkerSlot::new(Arc::clone(&self.active_workers));
            tokio::spawn(run_worker(queue, tx, slot));
        } else {
            tracing::warn!(
                "dispatch_single: MAX_CONCURRENT ({}) reached, dropping job — caller should retry",
                MAX_CONCURRENT
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
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
    fn round_robin_split_distributes_evenly() {
        // 7 jobs, 5 workers → sizes [2, 2, 1, 1, 1]
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
        // First bucket should always get jobs 0, n_workers, 2*n_workers, …
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
