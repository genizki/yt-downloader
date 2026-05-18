//! Tauri command bridge between the frontend (TypeScript/React) and the
//! GUI-independent service layer ([`AppService`]).
//!
//! Each `#[tauri::command]` function delegates into the shared [`AppService`].

use serde::Serialize;
use tauri::State;

use crate::api::types::{VideoId, YouTubeVideo};
use crate::download::progress::DownloadPhase;
use crate::service::{AppService, SearchStatus};
use crate::settings::AppSettings;

/// Shared, interior-mutable wrapper held by Tauri's managed state.
pub struct ServiceState(pub AppService);

#[derive(Serialize)]
pub struct VideoPhase {
    pub id: String,
    pub phase: DownloadPhase,
}

#[tauri::command]
pub fn get_settings(state: State<'_, ServiceState>) -> AppSettings {
    state.0.settings()
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, ServiceState>,
    settings: AppSettings,
) -> Result<(), String> {
    state.0.update_settings(settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_results(state: State<'_, ServiceState>) -> Vec<YouTubeVideo> {
    state.0.results()
}

#[tauri::command]
pub fn get_phases(state: State<'_, ServiceState>) -> Vec<VideoPhase> {
    state
        .0
        .download_phases()
        .iter()
        .map(|(id, phase)| VideoPhase {
            id: id.0.clone(),
            phase: phase.clone(),
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStatusDto {
    pub searched: bool,
    pub last_query: String,
    pub auto_downloading_count: Option<usize>,
    pub no_api_key: bool,
}

impl From<SearchStatus> for SearchStatusDto {
    fn from(value: SearchStatus) -> Self {
        Self {
            searched: value.searched,
            last_query: value.last_query,
            auto_downloading_count: value.auto_downloading_count,
            no_api_key: value.no_api_key,
        }
    }
}

#[tauri::command]
pub fn get_search_status(state: State<'_, ServiceState>) -> SearchStatusDto {
    state.0.get_search_status().into()
}

#[tauri::command]
pub fn submit_search(state: State<'_, ServiceState>, query: String) {
    state.0.submit_search(query);
}

#[tauri::command]
pub fn clear_search(state: State<'_, ServiceState>) {
    state.0.clear_search();
}

#[tauri::command]
pub fn download_single(state: State<'_, ServiceState>, video_id: String) {
    let _ = state.0.download_single(VideoId(video_id));
}
