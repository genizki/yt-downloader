use std::collections::HashMap;

use crate::api::types::{VideoId, YouTubeVideo};
use crate::download::progress::DownloadPhase;

pub trait DownloadInterface {
    fn results(&self) -> &[YouTubeVideo];
    fn download_phases(&self) -> &HashMap<VideoId, DownloadPhase>;
    fn download_phase(&self, id: &VideoId) -> Option<&DownloadPhase>;
    fn is_selected(&self, id: &VideoId) -> bool;
    fn selected_count(&self) -> usize;
    fn last_query(&self) -> &str;
    fn searched(&self) -> bool;

    fn submit_search(&mut self, query: String);
    fn clear_search(&mut self);
    fn poll_progress(&mut self) -> bool;
    fn poll_search(&mut self) -> bool;
    fn download_single(&mut self, video_id: VideoId);
    fn download_selected(&mut self);
    fn toggle_selected(&mut self, video_id: VideoId, selected: bool);
}
