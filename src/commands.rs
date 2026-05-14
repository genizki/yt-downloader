//! Tauri command bridge between the frontend (TypeScript/React) and the
//! GUI-independent service layer ([`AppService`]).
//!
//! Each `#[tauri::command]` function locks the shared [`AppService`] (held in
//! Tauri-managed state behind a [`std::sync::Mutex`]) and delegates to one of
//! the two interface traits defined in [`crate::service`].

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::api::types::{VideoId, YouTubeVideo};
use crate::download::progress::DownloadPhase;
use crate::service::{AppService, DownloadInterface, SettingsInterface};
use crate::settings::AppSettings;

/// Shared, interior-mutable wrapper held by Tauri's managed state.
pub struct ServiceState(pub Mutex<AppService>);

#[derive(Serialize)]
pub struct VideoPhase {
    pub id: String,
    pub phase: PhaseDto,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PhaseDto {
    Queued,
    Downloading { progress: f32 },
    PostProcessing,
    Moving,
    Done,
    Failed { error: String },
}

impl From<&DownloadPhase> for PhaseDto {
    fn from(p: &DownloadPhase) -> Self {
        match p {
            DownloadPhase::Queued => PhaseDto::Queued,
            DownloadPhase::Downloading(progress) => PhaseDto::Downloading {
                progress: *progress,
            },
            DownloadPhase::PostProcessing => PhaseDto::PostProcessing,
            DownloadPhase::Moving => PhaseDto::Moving,
            DownloadPhase::Done => PhaseDto::Done,
            DownloadPhase::Failed(error) => PhaseDto::Failed {
                error: error.clone(),
            },
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, ServiceState>) -> AppSettings {
    state.0.lock().unwrap().settings().clone()
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, ServiceState>,
    settings: AppSettings,
) -> Result<(), String> {
    let mut s = state.0.lock().unwrap();
    *s.settings_mut() = settings;
    s.save_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_results(state: State<'_, ServiceState>) -> Vec<YouTubeVideo> {
    state.0.lock().unwrap().results().to_vec()
}

#[tauri::command]
pub fn get_phases(state: State<'_, ServiceState>) -> Vec<VideoPhase> {
    let s = state.0.lock().unwrap();
    s.download_phases()
        .iter()
        .map(|(id, phase)| VideoPhase {
            id: id.0.clone(),
            phase: PhaseDto::from(phase),
        })
        .collect()
}

#[tauri::command]
pub fn get_selected(state: State<'_, ServiceState>) -> Vec<String> {
    let s = state.0.lock().unwrap();
    s.results()
        .iter()
        .filter(|v| s.is_selected(&v.id))
        .map(|v| v.id.0.clone())
        .collect()
}

#[tauri::command]
pub fn submit_search(state: State<'_, ServiceState>, query: String) {
    state.0.lock().unwrap().submit_search(query);
}

#[tauri::command]
pub fn clear_search(state: State<'_, ServiceState>) {
    state.0.lock().unwrap().clear_search();
}

#[tauri::command]
pub fn poll(state: State<'_, ServiceState>) -> PollSnapshot {
    let mut s = state.0.lock().unwrap();
    s.poll_progress();
    s.poll_search();
    PollSnapshot {
        searched: s.searched(),
        last_query: s.last_query().to_string(),
        results: s.results().to_vec(),
        phases: s
            .download_phases()
            .iter()
            .map(|(id, phase)| VideoPhase {
                id: id.0.clone(),
                phase: PhaseDto::from(phase),
            })
            .collect(),
        selected: s
            .results()
            .iter()
            .filter(|v| s.is_selected(&v.id))
            .map(|v| v.id.0.clone())
            .collect(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollSnapshot {
    pub searched: bool,
    pub last_query: String,
    pub results: Vec<YouTubeVideo>,
    pub phases: Vec<VideoPhase>,
    pub selected: Vec<String>,
}

#[tauri::command]
pub fn toggle_selected(state: State<'_, ServiceState>, video_id: String, selected: bool) {
    state
        .0
        .lock()
        .unwrap()
        .toggle_selected(VideoId(video_id), selected);
}

#[tauri::command]
pub fn download_single(state: State<'_, ServiceState>, video_id: String) {
    state.0.lock().unwrap().download_single(VideoId(video_id));
}

#[tauri::command]
pub fn download_selected(state: State<'_, ServiceState>) {
    state.0.lock().unwrap().download_selected();
}

// Silence the unused-import lint when the HashMap-typed helper is not used.
#[allow(dead_code)]
fn _phase_map_placeholder() -> HashMap<VideoId, DownloadPhase> {
    HashMap::new()
}
