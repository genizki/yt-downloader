//! Domain events emitted by [`crate::service::AppService`].
//!
//! See `CONTEXT.md` → "AppEvent" for the load-bearing definition. Every
//! state-changing operation on `AppService` emits exactly one `AppEvent`
//! describing what happened. Events flow out via a `tokio::sync::broadcast`
//! channel so multiple subscribers (Tauri-push task, tests, future logger)
//! can consume the same stream.
//!
//! Events are **past-tense facts**: never imperative. Use [`Action`] (when
//! it lands) for user-initiated intents.

#![allow(dead_code)]

use serde::Serialize;

use crate::api::parser::SearchKind;
use crate::api::types::VideoId;
use crate::download::progress::DownloadPhase;

/// Reason a download dispatch was rejected. Maps from
/// [`crate::download::dispatcher::DispatchOutcome`] non-`Queued` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RejectionReason {
    /// The video already has a `Queued`, `Downloading`, or `PostProcessing`
    /// phase entry.
    AlreadyInFlight,
    /// `MAX_CONCURRENT` workers are running; the manager dropped the job.
    NoSlot,
}

/// One past-tense fact emitted by the service. Subscribers fold these into
/// their own read models (frontend, tests, push bridge).
///
/// `Eq` is not derivable because `DownloadPhase::Downloading { progress:
/// f32 }` carries a float; `PartialEq` is enough for the tests we run.
///
/// Wire format: `#[serde(tag = "kind", rename_all = "camelCase")]`.
/// Frontend listens on the `"app-event"` Tauri channel and pattern-matches
/// on `payload.kind`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AppEvent {
    /// User submitted a search query. `has_api_key` indicates whether an
    /// HTTP request will follow; `playlist_auto` indicates whether the
    /// pending resolution will land in `AutoDownloading`.
    SearchSubmitted {
        query: String,
        search_kind: SearchKind,
        has_api_key: bool,
        playlist_auto: bool,
    },
    /// A YouTube API response arrived and the state transitioned to
    /// `Showing { results }`. `count` is the row count (0 is a valid empty
    /// result).
    SearchResolved { count: usize },
    /// `clear_search` returned the state to `Idle`.
    SearchCleared,
    /// A playlist was auto-dispatched; `count` jobs were enqueued.
    AutoDownloadStarted { count: usize },
    /// A worker reported a new phase for `video_id`. Sourced from the
    /// worker `ProgressEvent` mpsc and re-emitted on the broadcast bus.
    PhaseChanged {
        video_id: VideoId,
        phase: DownloadPhase,
    },
    /// A user-initiated dispatch was rejected. Accepted dispatches do not
    /// emit a dedicated event — the worker will follow up with
    /// `PhaseChanged(Queued)`.
    DownloadRejected {
        video_id: VideoId,
        reason: RejectionReason,
    },
    /// Settings were replaced and persisted (whether or not the write
    /// succeeded — failure is reported through the original return path,
    /// not through the bus).
    SettingsUpdated,
}

impl AppEvent {
    /// Convert a worker-emitted `ProgressEvent` into the bus-shaped
    /// `PhaseChanged` event. Lives here so the worker module stays free of
    /// the domain-wide `AppEvent` dependency.
    pub fn from_progress(event: crate::download::progress::ProgressEvent) -> Self {
        Self::PhaseChanged {
            video_id: event.video_id,
            phase: event.phase,
        }
    }
}

/// Name of the Tauri channel that carries every [`AppEvent`] to the
/// frontend. Frontend code uses `listen('app-event', cb)`.
pub const APP_EVENT_CHANNEL: &str = "app-event";

/// Bridge loop: read every event off the broadcast bus and emit it on the
/// Tauri `"app-event"` channel. Returns when the bus closes (service
/// dropped). Logs and continues on lagged subscribers and emit failures.
pub async fn bridge_to_tauri<R: tauri::Runtime>(
    handle: tauri::AppHandle<R>,
    mut rx: tokio::sync::broadcast::Receiver<AppEvent>,
) {
    use tauri::Emitter;
    use tokio::sync::broadcast::error::RecvError;

    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Err(e) = handle.emit(APP_EVENT_CHANNEL, &event) {
                    tracing::warn!(?e, "failed to emit app-event");
                }
            }
            Err(RecvError::Lagged(n)) => {
                tracing::warn!(missed = n, "app-event bus subscriber lagged");
            }
            Err(RecvError::Closed) => {
                tracing::debug!("app-event bus closed, bridge exiting");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_event_maps_to_phase_changed() {
        let pe = crate::download::progress::ProgressEvent {
            video_id: VideoId::new("abc"),
            phase: DownloadPhase::Downloading { progress: 0.5 },
        };
        let ev = AppEvent::from_progress(pe);
        assert_eq!(
            ev,
            AppEvent::PhaseChanged {
                video_id: VideoId::new("abc"),
                phase: DownloadPhase::Downloading { progress: 0.5 },
            }
        );
    }

    #[test]
    fn rejection_reason_is_copy() {
        let r = RejectionReason::AlreadyInFlight;
        let _ = r;
        assert_eq!(r, RejectionReason::AlreadyInFlight);
    }
}
