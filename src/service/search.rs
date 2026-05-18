//! Search lifecycle state machine.
//!
//! Async task wiring is intentionally **not** part of this enum so the state
//! remains pure data (derivable `Eq`, `Debug`, `Clone`) and the transition
//! functions can be unit-tested without spawning tokio tasks.
//! See `CONTEXT.md` → "SearchState" for the domain definition.
//!
//! ```text
//!                  ┌──────────────────────────────────────┐
//!                  │                                      │
//!                  ▼                                      │
//!   ┌─── Idle ──── submit (no key) ─── NoApiKey { query } │
//!   │                │                                    │
//!   │                └── submit (key set) ─── Pending {…} │
//!   │                                          │          │
//!   │                                          ├── resolved (regular) ─── Showing { … }
//!   │                                          │                              │
//!   │                                          └── resolved (playlist_auto)   │
//!   │                                                  │                      │
//!   │                                                  ▼                      │
//!   │                                          AutoDownloading { count }      │
//!   │                                                  │                      │
//!   └──────────────────── clear() ◀────────────────────┴──────────────────────┘
//! ```

#![allow(dead_code)]

use tokio::sync::broadcast;

use super::events::AppEvent;
use crate::api::parser::{classify, SearchKind};
use crate::api::types::YouTubeVideo;

/// One of five mutually-exclusive search-lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SearchState {
    #[default]
    Idle,
    NoApiKey { query: String },
    Pending {
        query: String,
        kind: SearchKind,
        /// `true` iff the kind is `PlaylistId` AND the user enabled
        /// `playlist_auto_download` in settings.
        playlist_auto: bool,
    },
    Showing { query: String, results: Vec<YouTubeVideo> },
    AutoDownloading { count: usize },
}

#[derive(Debug)]
pub struct Resolved {
    pub next: SearchState,
    pub auto_download_videos: Option<Vec<YouTubeVideo>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSearch {
    pub query: String,
    pub kind: SearchKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchStatusView {
    pub searched: bool,
    pub last_query: String,
    pub auto_downloading_count: Option<usize>,
    pub no_api_key: bool,
}

#[derive(Debug)]
pub struct ResolvedOutcome {
    pub auto_download_videos: Option<Vec<YouTubeVideo>>,
    pub result_count: usize,
}

pub struct SearchHandler {
    state: SearchState,
    event_tx: broadcast::Sender<AppEvent>,
}

impl SearchHandler {
    pub fn new(event_tx: broadcast::Sender<AppEvent>) -> Self {
        Self {
            state: SearchState::Idle,
            event_tx,
        }
    }

    pub fn submit(
        &mut self,
        query: String,
        has_api_key: bool,
        playlist_auto_setting: bool,
    ) -> Option<PendingSearch> {
        let kind = classify(&query);
        let playlist_auto = matches!(kind, SearchKind::PlaylistId(_)) && playlist_auto_setting;
        self.emit(AppEvent::SearchSubmitted {
            query: query.clone(),
            search_kind: kind.clone(),
            has_api_key,
            playlist_auto,
        });
        self.state = SearchState::submitted(
            query.clone(),
            kind.clone(),
            has_api_key,
            playlist_auto_setting,
        );
        if has_api_key {
            Some(PendingSearch { query, kind })
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.state = SearchState::cleared();
        self.emit(AppEvent::SearchCleared);
    }

    pub fn apply_resolved(&mut self, results: Vec<YouTubeVideo>) -> ResolvedOutcome {
        let current = std::mem::take(&mut self.state);
        let transition = current.resolved(results);
        self.state = transition.next;
        let result_count = self.state.results().len();
        if let Some(videos) = transition.auto_download_videos {
            let count = videos.len();
            self.emit(AppEvent::AutoDownloadStarted { count });
            ResolvedOutcome {
                auto_download_videos: Some(videos),
                result_count,
            }
        } else {
            self.emit(AppEvent::SearchResolved { count: result_count });
            ResolvedOutcome {
                auto_download_videos: None,
                result_count,
            }
        }
    }

    pub fn results(&self) -> &[YouTubeVideo] {
        self.state.results()
    }

    pub fn status(&self) -> SearchStatusView {
        SearchStatusView {
            searched: self.state.searched(),
            last_query: self.state.last_query().to_string(),
            auto_downloading_count: self.state.auto_download_count(),
            no_api_key: self.state.no_api_key(),
        }
    }

    pub fn state(&self) -> &SearchState {
        &self.state
    }

    #[cfg(test)]
    pub fn set_state_for_test(&mut self, next: SearchState) {
        self.state = next;
    }

    fn emit(&self, event: AppEvent) {
        let _ = self.event_tx.send(event);
    }
}

impl SearchState {
    pub fn submitted(
        query: String,
        kind: SearchKind,
        api_key_present: bool,
        playlist_auto_setting: bool,
    ) -> Self {
        if !api_key_present {
            return Self::NoApiKey { query };
        }
        let playlist_auto = matches!(kind, SearchKind::PlaylistId(_)) && playlist_auto_setting;
        Self::Pending {
            query,
            kind,
            playlist_auto,
        }
    }

    pub fn resolved(self, results: Vec<YouTubeVideo>) -> Resolved {
        match self {
            Self::Pending {
                playlist_auto: true,
                ..
            } => {
                let count = results.len();
                Resolved {
                    next: Self::AutoDownloading { count },
                    auto_download_videos: Some(results),
                }
            }
            Self::Pending { query, .. } => Resolved {
                next: Self::Showing { query, results },
                auto_download_videos: None,
            },
            other => Resolved {
                next: other,
                auto_download_videos: None,
            },
        }
    }

    pub fn cleared() -> Self {
        Self::Idle
    }

    pub fn searched(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    pub fn last_query(&self) -> &str {
        match self {
            Self::Idle | Self::AutoDownloading { .. } => "",
            Self::NoApiKey { query }
            | Self::Pending { query, .. }
            | Self::Showing { query, .. } => query,
        }
    }

    pub fn results(&self) -> &[YouTubeVideo] {
        match self {
            Self::Showing { results, .. } => results,
            _ => &[],
        }
    }

    pub fn auto_download_count(&self) -> Option<usize> {
        match self {
            Self::AutoDownloading { count } => Some(*count),
            _ => None,
        }
    }

    pub fn no_api_key(&self) -> bool {
        matches!(self, Self::NoApiKey { .. })
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::VideoId;
    use chrono::{DateTime, Utc};
    use tokio::sync::broadcast;

    fn video(id: &str) -> YouTubeVideo {
        YouTubeVideo {
            id: VideoId::new(id),
            title: id.into(),
            channel: "ch".into(),
            duration_seconds: 1,
            views: 0,
            published_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            thumbnail_url: String::new(),
        }
    }

    #[test]
    fn idle_is_default_and_not_searched() {
        let s = SearchState::default();
        assert_eq!(s, SearchState::Idle);
        assert!(!s.searched());
        assert_eq!(s.last_query(), "");
        assert!(s.results().is_empty());
        assert_eq!(s.auto_download_count(), None);
        assert!(!s.no_api_key());
    }

    #[test]
    fn submitted_without_key_lands_in_no_api_key() {
        let s = SearchState::submitted(
            "cats".into(),
            SearchKind::Query("cats".into()),
            false,
            false,
        );
        assert_eq!(
            s,
            SearchState::NoApiKey {
                query: "cats".into()
            }
        );
        assert!(s.searched());
        assert_eq!(s.last_query(), "cats");
        assert!(s.no_api_key());
    }

    #[test]
    fn submitted_query_with_key_is_pending() {
        let s = SearchState::submitted(
            "cats".into(),
            SearchKind::Query("cats".into()),
            true,
            true,
        );
        match s {
            SearchState::Pending {
                query,
                playlist_auto,
                ..
            } => {
                assert_eq!(query, "cats");
                assert!(!playlist_auto, "Query is not a playlist");
            }
            other => panic!("expected Pending, got {other:?}"),
        }
    }

    #[test]
    fn submitted_playlist_with_setting_off_is_pending_without_auto() {
        let s = SearchState::submitted(
            "PL123".into(),
            SearchKind::PlaylistId("PL123".into()),
            true,
            false,
        );
        match s {
            SearchState::Pending { playlist_auto, .. } => assert!(!playlist_auto),
            other => panic!("expected Pending, got {other:?}"),
        }
    }

    #[test]
    fn submitted_playlist_with_setting_on_enables_auto() {
        let s = SearchState::submitted(
            "PL123".into(),
            SearchKind::PlaylistId("PL123".into()),
            true,
            true,
        );
        match s {
            SearchState::Pending { playlist_auto, .. } => assert!(playlist_auto),
            other => panic!("expected Pending, got {other:?}"),
        }
    }

    #[test]
    fn resolved_pending_query_transitions_to_showing() {
        let pending =
            SearchState::submitted("cats".into(), SearchKind::Query("cats".into()), true, false);
        let r = pending.resolved(vec![video("a"), video("b")]);
        assert_eq!(
            r.next,
            SearchState::Showing {
                query: "cats".into(),
                results: vec![video("a"), video("b")],
            }
        );
        assert!(r.auto_download_videos.is_none());
    }

    #[test]
    fn resolved_pending_playlist_auto_transitions_to_auto_downloading() {
        let pending = SearchState::submitted(
            "PL123".into(),
            SearchKind::PlaylistId("PL123".into()),
            true,
            true,
        );
        let videos = vec![video("a"), video("b"), video("c")];
        let r = pending.resolved(videos.clone());
        assert_eq!(r.next, SearchState::AutoDownloading { count: 3 });
        assert_eq!(r.auto_download_videos, Some(videos));
    }

    #[test]
    fn resolved_outside_pending_is_a_noop() {
        let r = SearchState::Idle.resolved(vec![video("a")]);
        assert_eq!(r.next, SearchState::Idle);
        assert!(r.auto_download_videos.is_none());
    }

    #[test]
    fn resolved_with_empty_results_lands_in_showing_with_empty_vec() {
        let pending =
            SearchState::submitted("void".into(), SearchKind::Query("void".into()), true, false);
        let r = pending.resolved(vec![]);
        assert_eq!(
            r.next,
            SearchState::Showing {
                query: "void".into(),
                results: vec![],
            }
        );
        assert!(r.next.searched());
        assert!(r.next.results().is_empty());
    }

    #[test]
    fn cleared_returns_to_idle_from_any_state() {
        assert_eq!(SearchState::cleared(), SearchState::Idle);
    }

    #[test]
    fn auto_downloading_has_no_query_or_results() {
        let s = SearchState::AutoDownloading { count: 7 };
        assert!(s.searched());
        assert_eq!(s.last_query(), "");
        assert!(s.results().is_empty());
        assert_eq!(s.auto_download_count(), Some(7));
    }

    #[test]
    fn handler_submit_without_key_emits_submitted_and_returns_no_pending() {
        let (tx, mut rx) = broadcast::channel(8);
        let mut handler = SearchHandler::new(tx);

        let pending = handler.submit("cats".into(), false, false);
        assert!(pending.is_none());
        assert_eq!(
            rx.try_recv().expect("event delivered"),
            AppEvent::SearchSubmitted {
                query: "cats".into(),
                search_kind: SearchKind::Query("cats".into()),
                has_api_key: false,
                playlist_auto: false,
            }
        );
        assert_eq!(
            handler.state(),
            &SearchState::NoApiKey {
                query: "cats".into()
            }
        );
    }

    #[test]
    fn handler_apply_resolved_auto_mode_emits_auto_download_started() {
        let (tx, mut rx) = broadcast::channel(8);
        let mut handler = SearchHandler::new(tx);
        handler.set_state_for_test(SearchState::Pending {
            query: "PL123".into(),
            kind: SearchKind::PlaylistId("PL123".into()),
            playlist_auto: true,
        });

        let videos = vec![video("a"), video("b")];
        let outcome = handler.apply_resolved(videos.clone());
        assert_eq!(outcome.auto_download_videos, Some(videos));
        assert_eq!(outcome.result_count, 0);
        assert_eq!(
            rx.try_recv().expect("event delivered"),
            AppEvent::AutoDownloadStarted { count: 2 }
        );
        assert_eq!(handler.state(), &SearchState::AutoDownloading { count: 2 });
    }
}
