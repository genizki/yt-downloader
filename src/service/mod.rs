//! GUI-independent service layer.
//!
//! All state lives in [`AppService`]; the frontend (Tauri commands) drives it
//! by calling methods and subscribing to events.
//!
//! See `docs/adr/0001-appservice-is-tauri-only.md` for why the previous
//! `SettingsInterface` / `DownloadInterface` traits were removed.
//!
//! The search lifecycle is modelled by [`search::SearchState`] (see
//! `CONTEXT.md`).

#![allow(dead_code)]

pub mod events;
pub mod search;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::api::types::{VideoId, YouTubeVideo};
use crate::api::youtube::YouTubeClient;
use crate::download::dispatcher::{DispatchOutcome, DownloadDispatcher};
use crate::download::progress::{DownloadPhase, ProgressEvent};
use crate::settings::persistence;
use crate::settings::AppSettings;

use events::{AppEvent, RejectionReason};
use search::{SearchHandler, SearchStatusView};

/// Capacity of the [`AppEvent`] broadcast bus. Subscribers that fall behind
/// by more than this many events miss the oldest ones; this is acceptable
/// for UI updates (the next snapshot/event will resync the picture).
const EVENT_BUS_CAPACITY: usize = 256;

pub struct AppService {
    inner: Arc<Mutex<AppServiceInner>>,
}

struct AppServiceInner {
    settings: AppSettings,
    download_phases: HashMap<VideoId, DownloadPhase>,
    search: SearchHandler,
    dispatcher: DownloadDispatcher,
    event_tx: broadcast::Sender<AppEvent>,
}

impl AppService {
    pub fn new() -> Self {
        let settings = persistence::load();
        let (tx, rx) = mpsc::channel(256);
        let dispatcher = DownloadDispatcher::new(tx);
        let (event_tx, _initial_rx) = broadcast::channel(EVENT_BUS_CAPACITY);
        let inner = Arc::new(Mutex::new(AppServiceInner {
            settings,
            download_phases: HashMap::new(),
            search: SearchHandler::new(event_tx.clone()),
            dispatcher,
            event_tx,
        }));

        Self::spawn_progress_bridge(inner.clone(), rx);

        Self { inner }
    }

    fn spawn_progress_bridge(
        inner: Arc<Mutex<AppServiceInner>>,
        mut progress_rx: mpsc::Receiver<ProgressEvent>,
    ) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            tracing::debug!("tokio runtime not available; skipping progress bridge spawn");
            return;
        };

        handle.spawn(async move {
            while let Some(event) = progress_rx.recv().await {
                let mut inner = inner.lock().unwrap();
                inner
                    .download_phases
                    .insert(event.video_id.clone(), event.phase.clone());
                inner.emit(AppEvent::from_progress(event));
            }
            tracing::debug!("progress channel closed; progress bridge exiting");
        });
    }

    fn with_inner<R>(&self, f: impl FnOnce(&mut AppServiceInner) -> R) -> R {
        let mut inner = self.inner.lock().unwrap();
        f(&mut inner)
    }

    fn with_inner_ref<R>(&self, f: impl FnOnce(&AppServiceInner) -> R) -> R {
        let inner = self.inner.lock().unwrap();
        f(&inner)
    }

    pub fn get_search_status(&self) -> SearchStatus {
        self.with_inner_ref(|inner| inner.search.status().into())
    }

    pub fn settings(&self) -> AppSettings {
        self.with_inner_ref(|inner| inner.settings.clone())
    }

    pub fn update_settings(&self, new: AppSettings) -> anyhow::Result<()> {
        self.with_inner(|inner| {
            tracing::debug!("updating settings and persisting");
            inner.settings = new;
            let result = persistence::save(&inner.settings);
            inner.emit(AppEvent::SettingsUpdated);
            result
        })
    }

    pub fn results(&self) -> Vec<YouTubeVideo> {
        self.with_inner_ref(|inner| inner.search.results().to_vec())
    }

    pub fn download_phases(&self) -> HashMap<VideoId, DownloadPhase> {
        self.with_inner_ref(|inner| inner.download_phases.clone())
    }

    pub fn submit_search(&self, query: String) {
        let mut task = None;

        self.with_inner(|inner| {
            // New search → cancel any in-flight downloads from the previous
            // search and drop their phase entries so the UI starts clean.
            inner.dispatcher.cancel_all();
            inner.download_phases.clear();

            let api_key_present = !inner.settings.youtube_api_key.is_empty();
            let playlist_auto_setting = inner.settings.playlist_auto_download;
            let pending = inner
                .search
                .submit(query.clone(), api_key_present, playlist_auto_setting);
            if let Some(pending) = pending {
                task = Some((inner.settings.youtube_api_key.clone(), pending, self.inner.clone()));
            } else {
                tracing::debug!("no YouTube API key — search skipped");
            }
        });

        if let Some((key, pending, inner)) = task {
            let Ok(handle) = tokio::runtime::Handle::try_current() else {
                tracing::warn!("tokio runtime not available; cannot resolve search");
                return;
            };
            handle.spawn(async move {
                let client = YouTubeClient::new(key);
                let results = client.resolve(&pending.kind).await.unwrap_or_default();
                tracing::debug!(count = results.len(), "YouTube API response received");

                let mut inner = inner.lock().unwrap();
                let outcome = inner.search.apply_resolved(results);
                if let Some(videos) = outcome.auto_download_videos {
                    inner.dispatch_auto_download(videos);
                }
            });
        }
    }

    pub fn clear_search(&self) {
        self.with_inner(|inner| {
            tracing::debug!("search cleared → returning to hero view");
            inner.dispatcher.cancel_all();
            inner.download_phases.clear();
            inner.search.clear();
        });
    }

    pub fn download_single(&self, video_id: VideoId) -> DispatchOutcome {
        self.with_inner(|inner| {
            let outcome = inner.dispatcher.dispatch_single(
                video_id.clone(),
                &inner.settings,
                &inner.download_phases,
            );
            tracing::debug!(?outcome, "download_single");
            if let Some(reason) = rejection_of(outcome) {
                inner.emit(AppEvent::DownloadRejected { video_id, reason });
            }
            outcome
        })
    }

    /// Subscribe to the [`AppEvent`] broadcast bus. The returned receiver
    /// starts at the next event; previously emitted events are not
    /// replayed. Drop the receiver to unsubscribe.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.with_inner_ref(|inner| inner.event_tx.subscribe())
    }
}

#[derive(Debug, Clone)]
pub struct SearchStatus {
    pub searched: bool,
    pub last_query: String,
    pub auto_downloading_count: Option<usize>,
    pub no_api_key: bool,
}

impl From<SearchStatusView> for SearchStatus {
    fn from(value: SearchStatusView) -> Self {
        Self {
            searched: value.searched,
            last_query: value.last_query,
            auto_downloading_count: value.auto_downloading_count,
            no_api_key: value.no_api_key,
        }
    }
}

impl AppServiceInner {
    /// Internal helper: send on the bus. `broadcast::send` errors when there
    /// are no live subscribers; that is expected (the service runs without
    /// a UI in tests) and is silently ignored.
    fn emit(&self, event: AppEvent) {
        let _ = self.event_tx.send(event);
    }

    fn dispatch_auto_download(&mut self, videos: Vec<YouTubeVideo>) {
        self.dispatcher.dispatch_auto(videos, &self.settings);
    }
}

impl Default for AppService {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a [`DispatchOutcome`] to a [`RejectionReason`]. `Queued` returns
/// `None` — accepted dispatches do not emit a dedicated event; the worker
/// will follow up with `PhaseChanged(Queued)`.
fn rejection_of(outcome: DispatchOutcome) -> Option<RejectionReason> {
    match outcome {
        DispatchOutcome::Queued => None,
        DispatchOutcome::AlreadyInFlight => Some(RejectionReason::AlreadyInFlight),
        DispatchOutcome::NoSlot => Some(RejectionReason::NoSlot),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an `AppService` with the API key forcibly cleared so
    /// `submit_search` does not spawn a tokio task. The on-disk settings
    /// file is read at construction time, but not written here.
    fn service_no_api_key() -> AppService {
        let s = AppService::new();
        s.with_inner(|inner| {
            inner.settings.youtube_api_key.clear();
        });
        s
    }

    #[test]
    fn subscribe_receives_search_cleared() {
        let s = service_no_api_key();
        let mut rx = s.subscribe();
        s.clear_search();
        let ev = rx.try_recv().expect("event delivered");
        assert_eq!(ev, AppEvent::SearchCleared);
    }

    #[test]
    fn submit_search_emits_search_submitted() {
        let s = service_no_api_key();
        let mut rx = s.subscribe();
        s.submit_search("cats".into());
        let ev = rx.try_recv().expect("event delivered");
        match ev {
            AppEvent::SearchSubmitted {
                query,
                has_api_key,
                playlist_auto,
                ..
            } => {
                assert_eq!(query, "cats");
                assert!(!has_api_key);
                assert!(!playlist_auto);
            }
            other => panic!("expected SearchSubmitted, got {other:?}"),
        }
    }

    #[test]
    fn no_subscribers_does_not_panic() {
        // Emitting onto a bus with no receivers must be silent, not error.
        let s = service_no_api_key();
        s.clear_search();
    }
}
