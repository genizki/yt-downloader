//! Progress types for download job lifecycle tracking.
//!
//! [`DownloadPhase`] describes the lifecycle of a single download job.
//! [`ProgressEvent`] is the message sent from a worker task to the UI thread
//! via a `tokio::sync::mpsc` channel.

#![allow(dead_code)]

use serde::Serialize;

use crate::api::types::VideoId;

/// Lifecycle phase of a single download job.
///
/// Serialized as a tagged union (`{kind: "downloading", progress: 0.5}`) so
/// the frontend can pattern-match on `kind` without a separate DTO.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DownloadPhase {
    Queued,
    Downloading { progress: f32 },
    PostProcessing,
    Moving,
    Done,
    Failed { error: String },
}

/// Event sent from a worker task to the UI thread via mpsc.
#[derive(Clone, Debug)]
pub struct ProgressEvent {
    pub video_id: VideoId,
    pub phase: DownloadPhase,
}
