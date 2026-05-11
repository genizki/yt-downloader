//! Progress types for download job lifecycle tracking.
//!
//! [`DownloadPhase`] describes the lifecycle of a single download job.
//! [`ProgressEvent`] is the message sent from a worker task to the UI thread
//! via a `tokio::sync::mpsc` channel.

#![allow(dead_code)]

use crate::api::types::VideoId;

/// Lifecycle phase of a single download job.
#[derive(Clone, Debug, PartialEq)]
pub enum DownloadPhase {
    Queued,
    Downloading(f32), // progress 0.0..=1.0
    PostProcessing,
    Moving,
    Done,
    Failed(String), // error message
}

/// Event sent from a worker task to the UI thread via mpsc.
#[derive(Clone, Debug)]
pub struct ProgressEvent {
    pub video_id: VideoId,
    pub phase: DownloadPhase,
}
