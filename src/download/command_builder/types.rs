//! Domain types for the yt-dlp command builder.
//!
//! Enums are designed so that mutually exclusive options are unrepresentable.
//! For example, [`Format`] makes "audio + video container" impossible at the
//! type level, and [`Cookies`] makes "browser + file" impossible. Conflicts
//! that span otherwise-independent fields are checked at runtime in
//! [`crate::download::command_builder::validate`].

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Input URLs for the command. At least one URL or a file must be present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Single(String),
    Multiple(Vec<String>),
    File(PathBuf),
}

/// Top-level format selection. Video and audio are mutually exclusive at the
/// type level so that audio-only callers cannot accidentally set a video
/// container, and vice versa. `Custom` is an escape hatch passed through
/// verbatim to yt-dlp's `-f` flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Format {
    Video {
        quality: VideoQuality,
        container: VideoContainer,
    },
    Audio {
        container: AudioContainer,
        /// yt-dlp `--audio-quality` value. Accepts either the 0..=9 VBR
        /// scale (0 = best) or a CBR kbps value (32..=512). The legacy
        /// `AppSettings::audio_bitrate` mapping uses kbps.
        quality: u16,
    },
    Custom(String),
}

/// Video quality presets. `Custom` carries a yt-dlp height/preset string
/// rendered verbatim into the `-f` selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoQuality {
    Best,
    P1080,
    P720,
    P480,
    Custom(String),
}

/// Video output containers accepted by `--merge-output-format`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoContainer {
    Mp4,
    Mkv,
    WebM,
}

impl VideoContainer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::WebM => "webm",
        }
    }
}

/// Audio output containers accepted by `--audio-format`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioContainer {
    Mp3,
    Aac,
    Flac,
    Opus,
    Wav,
    M4a,
    Alac,
    Aiff,
}

impl AudioContainer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Opus => "opus",
            Self::Wav => "wav",
            Self::M4a => "m4a",
            Self::Alac => "alac",
            Self::Aiff => "aiff",
        }
    }
}

/// Cookies source. Type-level exclusivity guarantees only one source can be
/// active at a time, fulfilling rule (4) of the task spec.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Cookies {
    #[default]
    None,
    File(PathBuf),
    Browser(BrowserKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserKind {
    Chrome,
    Firefox,
    Edge,
    Safari,
    Brave,
    Chromium,
    Opera,
    Vivaldi,
}

impl BrowserKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Edge => "edge",
            Self::Safari => "safari",
            Self::Brave => "brave",
            Self::Chromium => "chromium",
            Self::Opera => "opera",
            Self::Vivaldi => "vivaldi",
        }
    }
}

/// Whether playlists in the URL should be downloaded.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PlaylistMode {
    #[default]
    Single,
    Playlist,
}

/// SponsorBlock action. `Remove` strips segments from the output, `Mark`
/// embeds them as chapters.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum SponsorBlock {
    #[default]
    None,
    Remove(Vec<Segment>),
    Mark(Vec<Segment>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Segment {
    Sponsor,
    Selfpromo,
    Interaction,
    Intro,
    Outro,
    Preview,
    MusicOfftopic,
    Filler,
}

impl Segment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sponsor => "sponsor",
            Self::Selfpromo => "selfpromo",
            Self::Interaction => "interaction",
            Self::Intro => "intro",
            Self::Outro => "outro",
            Self::Preview => "preview",
            Self::MusicOfftopic => "music_offtopic",
            Self::Filler => "filler",
        }
    }
}

/// Recode or remux post-processing. Mutually exclusive by construction.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum RecodeOrRemux {
    #[default]
    None,
    /// Re-encode to the named container (loses fidelity).
    Recode(String),
    /// Remux to the named container without re-encoding.
    Remux(String),
}

// --- Settings sub-structs (persisted in AppSettings) ----------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSettings {
    #[serde(default)]
    pub mode: PlaylistModeSerde,
    #[serde(default)]
    pub playlist_start: Option<u32>,
    #[serde(default)]
    pub playlist_end: Option<u32>,
    #[serde(default)]
    pub playlist_items: Option<String>,
    #[serde(default)]
    pub max_downloads: Option<u32>,
    #[serde(default)]
    pub match_title: Option<String>,
    #[serde(default)]
    pub reject_title: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub datebefore: Option<String>,
    #[serde(default)]
    pub dateafter: Option<String>,
    #[serde(default)]
    pub min_views: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlaylistModeSerde {
    #[default]
    Single,
    Playlist,
}

impl From<PlaylistModeSerde> for PlaylistMode {
    fn from(m: PlaylistModeSerde) -> Self {
        match m {
            PlaylistModeSerde::Single => Self::Single,
            PlaylistModeSerde::Playlist => Self::Playlist,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSettings {
    #[serde(default)]
    pub rate_limit: Option<String>,
    #[serde(default)]
    pub concurrent_fragments: Option<u32>,
    #[serde(default)]
    pub retries: Option<u32>,
    #[serde(default)]
    pub fragment_retries: Option<u32>,
    #[serde(default)]
    pub cookies: CookiesSerde,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default)]
    pub source_address: Option<String>,
    #[serde(default)]
    pub geo_bypass: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum CookiesSerde {
    #[default]
    None,
    File {
        path: PathBuf,
    },
    Browser {
        browser: String,
    },
}

impl CookiesSerde {
    pub fn to_cookies(&self) -> Cookies {
        match self {
            Self::None => Cookies::None,
            Self::File { path } => Cookies::File(path.clone()),
            Self::Browser { browser } => match browser.to_lowercase().as_str() {
                "chrome" => Cookies::Browser(BrowserKind::Chrome),
                "firefox" => Cookies::Browser(BrowserKind::Firefox),
                "edge" => Cookies::Browser(BrowserKind::Edge),
                "safari" => Cookies::Browser(BrowserKind::Safari),
                "brave" => Cookies::Browser(BrowserKind::Brave),
                "chromium" => Cookies::Browser(BrowserKind::Chromium),
                "opera" => Cookies::Browser(BrowserKind::Opera),
                "vivaldi" => Cookies::Browser(BrowserKind::Vivaldi),
                _ => Cookies::None,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExtras {
    #[serde(default)]
    pub add_metadata: bool,
    #[serde(default)]
    pub write_thumbnail: bool,
    #[serde(default)]
    pub write_info_json: bool,
    #[serde(default)]
    pub write_description: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubtitlesSettings {
    #[serde(default)]
    pub write_auto_subs: bool,
    #[serde(default)]
    pub sub_langs: Vec<String>,
    #[serde(default)]
    pub sub_format: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessing {
    #[serde(default)]
    pub sponsorblock: SponsorBlockSerde,
    #[serde(default)]
    pub split_chapters: bool,
    #[serde(default)]
    pub download_sections: Option<String>,
    #[serde(default)]
    pub exec: Option<String>,
    #[serde(default)]
    pub ffmpeg_location: Option<PathBuf>,
    #[serde(default)]
    pub postprocessor_args: Option<String>,
    #[serde(default)]
    pub recode_or_remux: RecodeOrRemuxSerde,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum SponsorBlockSerde {
    #[default]
    None,
    Remove {
        segments: Vec<String>,
    },
    Mark {
        segments: Vec<String>,
    },
}

impl SponsorBlockSerde {
    pub fn to_sponsorblock(&self) -> SponsorBlock {
        match self {
            Self::None => SponsorBlock::None,
            Self::Remove { segments } => SponsorBlock::Remove(parse_segments(segments)),
            Self::Mark { segments } => SponsorBlock::Mark(parse_segments(segments)),
        }
    }
}

fn parse_segments(names: &[String]) -> Vec<Segment> {
    names
        .iter()
        .filter_map(|n| match n.to_lowercase().as_str() {
            "sponsor" => Some(Segment::Sponsor),
            "selfpromo" => Some(Segment::Selfpromo),
            "interaction" => Some(Segment::Interaction),
            "intro" => Some(Segment::Intro),
            "outro" => Some(Segment::Outro),
            "preview" => Some(Segment::Preview),
            "music_offtopic" | "musicofftopic" => Some(Segment::MusicOfftopic),
            "filler" => Some(Segment::Filler),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum RecodeOrRemuxSerde {
    #[default]
    None,
    Recode {
        container: String,
    },
    Remux {
        container: String,
    },
}

impl RecodeOrRemuxSerde {
    pub fn to_recode_or_remux(&self) -> RecodeOrRemux {
        match self {
            Self::None => RecodeOrRemux::None,
            Self::Recode { container } => RecodeOrRemux::Recode(container.clone()),
            Self::Remux { container } => RecodeOrRemux::Remux(container.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MiscSettings {
    #[serde(default)]
    pub simulate: bool,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub quiet: bool,
    #[serde(default)]
    pub no_warnings: bool,
    #[serde(default)]
    pub sleep_interval: Option<u32>,
}
