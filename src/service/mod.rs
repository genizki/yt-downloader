//! GUI-independent service layer.
//!
//! This module exposes the core operations (search, download, settings) as a
//! pure async API that any frontend (egui, web, Swift via FFI) can consume.
//! All state lives in [`AppService`]; the frontend only needs to poll events
//! and call methods.
//!
//! Two trait interfaces decouple the UI from the concrete service:
//! - [`SettingsInterface`] — configuration read/write/persist
//! - [`DownloadInterface`] — search, results, selection, download dispatch

#![allow(dead_code)]

pub mod download_interface;
pub mod settings_interface;

pub use download_interface::DownloadInterface;
pub use settings_interface::SettingsInterface;

use std::collections::{HashMap, HashSet};

use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::api::parser::{classify, SearchKind};
use crate::api::types::{VideoId, YouTubeVideo};
use crate::api::youtube::YouTubeClient;
use crate::download::manager::DownloadManager;
use crate::download::progress::{DownloadPhase, ProgressEvent};
use crate::download::worker::DownloadJob;
use crate::settings::persistence;
use crate::settings::AppSettings;

/// GUI-independent application service.
///
/// Owns all runtime state related to search results, downloads, and settings.
/// A frontend drives this by calling methods and draining events.
pub struct AppService {
    settings: AppSettings,
    results: Vec<YouTubeVideo>,
    download_phases: HashMap<VideoId, DownloadPhase>,
    selected: HashSet<VideoId>,
    last_query: String,
    searched: bool,

    manager: DownloadManager,
    progress_rx: mpsc::Receiver<ProgressEvent>,
    search_rx: Option<oneshot::Receiver<Vec<YouTubeVideo>>>,
    playlist_mode_active: bool,
}

impl AppService {
    pub fn new() -> Self {
        let settings = persistence::load();
        let (tx, rx) = mpsc::channel(256);
        let manager = DownloadManager::new(tx);
        Self {
            settings,
            results: Vec::new(),
            download_phases: HashMap::new(),
            selected: HashSet::new(),
            last_query: String::new(),
            searched: false,
            manager,
            progress_rx: rx,
            search_rx: None,
            playlist_mode_active: false,
        }
    }

    pub fn search_pending(&self) -> bool {
        self.search_rx.is_some()
    }

    fn auto_download_playlist(&mut self, videos: Vec<YouTubeVideo>) {
        if self.settings.playlist_auto_download {
            let count = videos.len();
            let temp_dir = crate::paths::temp_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("yt-dlp-gui"));
            let jobs: Vec<DownloadJob> = videos
                .into_iter()
                .map(|v| DownloadJob {
                    video_id: v.id,
                    settings: self.settings.clone(),
                    temp_dir: temp_dir.clone(),
                })
                .collect();
            self.manager.dispatch_batch(jobs);
            self.last_query = format!("Downloading {} playlist items\u{2026}", count);
        } else {
            self.results = videos;
        }
    }
}

impl SettingsInterface for AppService {
    fn settings(&self) -> &AppSettings {
        &self.settings
    }

    fn settings_mut(&mut self) -> &mut AppSettings {
        &mut self.settings
    }

    fn save_settings(&self) -> anyhow::Result<()> {
        tracing::debug!("saving settings to disk");
        persistence::save(&self.settings)?;
        Ok(())
    }
}

impl DownloadInterface for AppService {
    fn results(&self) -> &[YouTubeVideo] {
        &self.results
    }

    fn download_phases(&self) -> &HashMap<VideoId, DownloadPhase> {
        &self.download_phases
    }

    fn download_phase(&self, id: &VideoId) -> Option<&DownloadPhase> {
        self.download_phases.get(id)
    }

    fn is_selected(&self, id: &VideoId) -> bool {
        self.selected.contains(id)
    }

    fn selected_count(&self) -> usize {
        self.selected.len()
    }

    fn last_query(&self) -> &str {
        &self.last_query
    }

    fn searched(&self) -> bool {
        self.searched
    }

    fn submit_search(&mut self, query: String) {
        self.searched = true;
        self.last_query = query;

        let kind = classify(&self.last_query);
        self.playlist_mode_active =
            matches!(kind, SearchKind::PlaylistId(_)) && self.settings.playlist_auto_download;

        tracing::debug!(
            query = %self.last_query,
            kind = ?kind,
            api_key_set = !self.settings.youtube_api_key.is_empty(),
            playlist_auto = self.playlist_mode_active,
            "search submitted"
        );

        if !self.settings.youtube_api_key.is_empty() {
            let (tx, rx) = oneshot::channel();
            self.search_rx = Some(rx);
            let key = self.settings.youtube_api_key.clone();
            tracing::debug!("spawning YouTube API request");
            tokio::spawn(async move {
                let client = YouTubeClient::new(key);
                let result = client.resolve(&kind).await.unwrap_or_default();
                tracing::debug!(count = result.len(), "YouTube API response received");
                let _ = tx.send(result);
            });
        } else {
            tracing::debug!("no YouTube API key — search skipped");
        }
    }

    fn clear_search(&mut self) {
        tracing::debug!("search cleared → returning to hero view");
        self.searched = false;
        self.results.clear();
        self.playlist_mode_active = false;
    }

    fn poll_progress(&mut self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.progress_rx.try_recv() {
            self.download_phases.insert(event.video_id, event.phase);
            changed = true;
        }
        changed
    }

    fn poll_search(&mut self) -> bool {
        if let Some(rx) = self.search_rx.as_mut() {
            if let Ok(results) = rx.try_recv() {
                tracing::debug!(count = results.len(), "search results delivered to UI");
                if self.playlist_mode_active {
                    self.auto_download_playlist(results);
                } else {
                    self.results = results;
                }
                self.playlist_mode_active = false;
                self.search_rx = None;
                return true;
            }
        }
        false
    }

    fn download_single(&mut self, video_id: VideoId) {
        if matches!(
            self.download_phases.get(&video_id),
            Some(DownloadPhase::Downloading(_))
                | Some(DownloadPhase::PostProcessing)
                | Some(DownloadPhase::Queued)
        ) {
            tracing::debug!(id = %video_id.0, "download_single: already in-flight, skipping");
            return;
        }
        tracing::debug!(id = %video_id.0, "download_single: queuing");
        if crate::debug::enabled() {
            crate::debug::log_ytdlp_command(&self.settings);
        }
        if let Ok(temp_dir) = crate::paths::temp_dir() {
            let job = DownloadJob {
                video_id,
                settings: self.settings.clone(),
                temp_dir,
            };
            self.manager.dispatch_single(job);
        }
    }

    fn download_selected(&mut self) {
        let temp_dir =
            crate::paths::temp_dir().unwrap_or_else(|_| std::env::temp_dir().join("yt-dlp-gui"));
        let jobs: Vec<DownloadJob> = self
            .selected
            .iter()
            .filter(|id| {
                !matches!(
                    self.download_phases.get(*id),
                    Some(DownloadPhase::Queued)
                        | Some(DownloadPhase::Downloading(_))
                        | Some(DownloadPhase::PostProcessing)
                )
            })
            .map(|id| DownloadJob {
                video_id: id.clone(),
                settings: self.settings.clone(),
                temp_dir: temp_dir.clone(),
            })
            .collect();
        tracing::debug!(count = jobs.len(), "download_selected: queuing batch");
        if crate::debug::enabled() && !jobs.is_empty() {
            crate::debug::log_ytdlp_command(&self.settings);
        }
        self.manager.dispatch_batch(jobs);
        self.selected.clear();
    }

    fn toggle_selected(&mut self, video_id: VideoId, selected: bool) {
        tracing::debug!(id = %video_id.0, selected, "toggle_selected");
        if selected {
            self.selected.insert(video_id);
        } else {
            self.selected.remove(&video_id);
        }
    }
}
