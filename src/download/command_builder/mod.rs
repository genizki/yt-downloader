//! Build the argument vector handed to `yt-dlp`.
//!
//! Public callers use [`build_args`]. Internally we keep a fluent builder that
//! covers all command sections and validation rules.

mod builder;
mod command;
mod template;
pub mod types;
mod validate;

#[cfg(test)]
mod tests;

use std::path::Path;

use crate::settings::AppSettings;

#[allow(unused_imports)]
pub use types::{
    AudioContainer, BrowserKind, Cookies, Format, Input, MetadataExtras, MiscSettings,
    NetworkSettings, PlaylistMode, PlaylistSettings, PostProcessing, RecodeOrRemux, Segment,
    SponsorBlock, SubtitlesSettings, VideoContainer, VideoQuality,
};
#[allow(unused_imports)]
pub use validate::BuildError;

pub fn build_args(
    settings: &AppSettings,
    video_id: &str,
    temp_dir: &Path,
    ffmpeg_location: Option<&Path>,
) -> Result<Vec<String>, BuildError> {
    builder::CommandBuilder::from_settings(settings, video_id, temp_dir)
        .ffmpeg_location_opt(ffmpeg_location)
        .build()
        .map(|c| {
            c.into_args()
                .into_iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        })
}
