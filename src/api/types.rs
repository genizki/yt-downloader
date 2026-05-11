//! Domain types returned by the YouTube API client.
//!
//! See `## Datenmodell` in the project plan. The `VideoId` newtype enforces
//! the uniqueness contract that downstream tickets (`#22` manager,
//! `#23/#24/#25` UI integrations) rely on when keying download state into a
//! `HashMap<VideoId, DownloadPhase>`.
//!
//! Note: `Duration` is **stored** as `duration_seconds: u64` for serde-clean
//! TOML/JSON round-trips (serde 1 has no built-in `std::time::Duration`
//! adapter). The accessor [`YouTubeVideo::duration`] hands out a real
//! `std::time::Duration` for ergonomic downstream consumption.
#![allow(dead_code)] // consumed by tickets #13/#23/#25 once they land

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Newtype wrapping a YouTube video ID.
///
/// `Hash + Eq` so it keys the global `HashMap<VideoId, DownloadPhase>` in the
/// download manager (single source of truth per the plan's
/// "Thread-Isolation" section).
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoId(pub String);

impl VideoId {
    /// Construct from anything string-like.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the wrapped ID as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VideoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Fully resolved YouTube video metadata, ready to be rendered as a result
/// row and used as a download job source.
///
/// Built by `YouTubeClient::resolve` after merging `search.list` /
/// `playlistItems.list` IDs against `videos.list` so `duration_seconds` is
/// always populated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouTubeVideo {
    pub id: VideoId,
    pub title: String,
    pub channel: String,
    /// Stored as raw seconds so serde works with no extra adapter; access
    /// via [`Self::duration`] when you want a `std::time::Duration`.
    pub duration_seconds: u64,
    pub views: u64,
    pub published_at: DateTime<Utc>,
    pub thumbnail_url: String,
}

impl YouTubeVideo {
    /// Convenience accessor: the video's runtime as a `std::time::Duration`.
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.duration_seconds)
    }
}
