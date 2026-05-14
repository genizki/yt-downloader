//! Build the argument vector handed to `yt-dlp`.
//!
//! The public entry point is [`CommandBuilder`], a fluent builder that covers
//! all nine sections from the project spec (Input, Format, Output, Playlist,
//! Network, Metadata, Subtitles, Post-Processing, Misc) plus the legacy
//! transport flags (`--newline`, `--progress-template`, `--no-warnings`,
//! `--max-filesize`, oauth2 auth) expected by
//! [`crate::download::yt_dlp::spawn_and_track`].
//!
//! Two entry points:
//! - [`CommandBuilder::new`] for manual construction (tests, future flows).
//! - [`CommandBuilder::from_settings`] for the existing
//!   `(AppSettings, video_id, temp_dir)` call shape used by `worker.rs` and
//!   `debug.rs`.

pub mod builder;
pub mod command;
pub mod template;
pub mod types;
pub mod validate;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use builder::CommandBuilder;
#[allow(unused_imports)]
pub use command::Command;
#[allow(unused_imports)]
pub use types::{
    AudioContainer, BrowserKind, Cookies, Format, Input, MetadataExtras, MiscSettings,
    NetworkSettings, PlaylistMode, PlaylistSettings, PostProcessing, RecodeOrRemux, Segment,
    SponsorBlock, SubtitlesSettings, VideoContainer, VideoQuality,
};
#[allow(unused_imports)]
pub use validate::BuildError;
