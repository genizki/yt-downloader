//! Settings data model.
//!
//! Mirrors the JSX defaults in `claude Design/yt-dlp/project/app.jsx`
//! (lines 14-51 for the `EXTRAS` definition, lines 561-580 for the default
//! settings struct). String-typed JSX options become Rust enums with
//! `serde(rename = "...")` so the on-disk TOML reads cleanly (`"1080p"`,
//! `"H.264"`, `"256 kbps"`, ...), independent of the Rust variant names.

// Most of these types will not be consumed until tickets #14 (settings UI)
// and #19 (command builder) land. Suppress the lint at the module level
// instead of sprinkling `#[allow(dead_code)]` on each item.
#![allow(dead_code)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Output container / audio codec selected in the Downloads section.
///
/// The first six are video containers; the rest are audio-only formats and
/// mean "the user wants a `-x` extraction in this codec".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    #[serde(rename = "MP4")]
    Mp4,
    #[serde(rename = "MKV")]
    Mkv,
    #[serde(rename = "WebM")]
    WebM,
    #[serde(rename = "MOV")]
    Mov,
    #[serde(rename = "MP3")]
    Mp3,
    #[serde(rename = "M4A")]
    M4a,
    #[serde(rename = "AAC")]
    Aac,
    #[serde(rename = "ALAC")]
    Alac,
    #[serde(rename = "AIFF")]
    Aiff,
    #[serde(rename = "FLAC")]
    Flac,
}

impl Format {
    /// Whether the selected `Format` triggers the audio-only path in the
    /// command builder. Mirrors the JSX `AUDIO_FORMATS` constant.
    pub fn is_audio_only(&self) -> bool {
        matches!(
            self,
            Self::Mp3 | Self::M4a | Self::Aac | Self::Alac | Self::Aiff | Self::Flac
        )
    }
}

/// Maximum video height the user wants. Serialized as `"360p"`, `"720p"`, ...
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    #[serde(rename = "360p")]
    P360,
    #[serde(rename = "720p")]
    P720,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "1440p")]
    P1440,
    #[serde(rename = "2160p")]
    P2160,
}

/// Preferred video codec. Serialized using the user-visible spelling
/// (`"H.264"`, `"H.265"`, `"VP9"`, `"AV1"`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Codec {
    #[serde(rename = "H.264")]
    H264,
    #[serde(rename = "H.265")]
    H265,
    #[serde(rename = "VP9")]
    Vp9,
    #[serde(rename = "AV1")]
    Av1,
}

/// Bitrate for the audio-only extraction path. Serialized as `"96 kbps"`...
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioBitrate {
    #[serde(rename = "96 kbps")]
    K96,
    #[serde(rename = "128 kbps")]
    K128,
    #[serde(rename = "192 kbps")]
    K192,
    #[serde(rename = "256 kbps")]
    K256,
    #[serde(rename = "320 kbps")]
    K320,
}

/// File-size cap for downloads. `NoLimit` is the JSX default ("No limit").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaxSize {
    #[serde(rename = "No limit")]
    NoLimit,
    #[serde(rename = "50 MB")]
    Mb50,
    #[serde(rename = "100 MB")]
    Mb100,
    #[serde(rename = "500 MB")]
    Mb500,
    #[serde(rename = "1 GB")]
    Gb1,
}

/// Network protocol preference forwarded to yt-dlp's `--downloader-args`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    #[serde(rename = "Auto")]
    Auto,
    #[serde(rename = "HTTPS")]
    Https,
    #[serde(rename = "HTTP")]
    Http,
    #[serde(rename = "HLS")]
    Hls,
    #[serde(rename = "DASH")]
    Dash,
}

/// UI theme. `System` follows the OS preference at runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[serde(rename = "Light")]
    Light,
    #[serde(rename = "Dark")]
    Dark,
    #[serde(rename = "System")]
    System,
}

/// Boolean post-processing toggles, mirroring the JSX `EXTRAS` array.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Extras {
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_chapters: bool,
    pub embed_subtitles: bool,
    pub write_subtitles: bool,
    pub skip_playlists: bool,
    pub restrict_names: bool,
}

/// Top-level persisted settings struct. Matches the JSX `settings` state
/// object plus a few Rust-side additions (`playlist_auto_download`,
/// `youtube_api_key`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub format: Format,
    pub quality: Quality,
    pub codec: Codec,
    pub audio_bitrate: AudioBitrate,
    pub download_path: PathBuf,
    pub max_size: MaxSize,
    pub protocol: Protocol,
    pub extras: Extras,
    pub theme: Theme,
    /// ISO-like language code, e.g. `"en"`, `"de"`.
    pub language: String,
    /// User-supplied bearer/cookie token for authenticated downloads.
    /// Empty string when unset.
    pub auth_token: String,
    /// When `true`, playlist URLs start downloading immediately instead of
    /// rendering an item list first. Toggle from the plan, not the JSX.
    pub playlist_auto_download: bool,
    /// Key for the YouTube Data v3 API (search.list / videos.list).
    /// Empty string when unset.
    pub youtube_api_key: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let youtube_api_key =
            std::env::var("ytapiv3key").unwrap_or_default();

        Self {
            format: Format::Mp4,
            quality: Quality::P1080,
            codec: Codec::H264,
            audio_bitrate: AudioBitrate::K256,
            download_path: crate::paths::default_downloads_dir(),
            max_size: MaxSize::NoLimit,
            protocol: Protocol::Auto,
            extras: Extras {
                embed_thumbnail: true,
                embed_metadata: true,
                ..Default::default()
            },
            theme: Theme::Light,
            language: "en".into(),
            auth_token: String::new(),
            playlist_auto_download: false,
            youtube_api_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_only_classification() {
        // Video containers must NOT be classified audio-only.
        for v in [Format::Mp4, Format::Mkv, Format::WebM, Format::Mov] {
            assert!(
                !v.is_audio_only(),
                "{v:?} is a video container, must not be audio_only"
            );
        }
        // Audio formats trigger the `-x` path.
        for a in [
            Format::Mp3,
            Format::M4a,
            Format::Aac,
            Format::Alac,
            Format::Aiff,
            Format::Flac,
        ] {
            assert!(a.is_audio_only(), "{a:?} should be audio_only");
        }
    }

    #[test]
    fn defaults_match_jsx_mockup() {
        let s = AppSettings::default();
        assert_eq!(s.format, Format::Mp4);
        assert_eq!(s.quality, Quality::P1080);
        assert_eq!(s.codec, Codec::H264);
        assert_eq!(s.audio_bitrate, AudioBitrate::K256);
        assert_eq!(s.max_size, MaxSize::NoLimit);
        assert_eq!(s.protocol, Protocol::Auto);
        assert_eq!(s.theme, Theme::Light);
        assert_eq!(s.language, "en");
        assert!(s.auth_token.is_empty());
        assert!(s.youtube_api_key.is_empty());
        assert!(!s.playlist_auto_download);
        assert!(s.extras.embed_thumbnail);
        assert!(s.extras.embed_metadata);
        assert!(!s.extras.embed_chapters);
        assert!(!s.extras.embed_subtitles);
        assert!(!s.extras.write_subtitles);
        assert!(!s.extras.skip_playlists);
        assert!(!s.extras.restrict_names);
    }

    #[test]
    fn toml_roundtrip_default() {
        let s = AppSettings::default();
        let toml_str = toml::to_string(&s).expect("serialize default settings");
        let parsed: AppSettings = toml::from_str(&toml_str).expect("re-parse settings");
        assert_eq!(s, parsed);
    }

    /// String-typed enums must round-trip through TOML using their JSX
    /// spellings (so users hand-editing `settings.toml` see "1080p", not
    /// "P1080"). We embed each enum into a tiny `Wrapper` to dodge TOML's
    /// "top-level must be a table" rule for primitive values.
    #[test]
    fn enum_string_roundtrip() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct W<T> {
            v: T,
        }

        fn check<T>(value: T, expected: &str)
        where
            T: PartialEq + std::fmt::Debug + Serialize + serde::de::DeserializeOwned,
        {
            let w = W { v: value };
            let s = toml::to_string(&w).expect("serialize");
            assert!(
                s.contains(expected),
                "expected on-disk repr {expected:?} in {s:?}"
            );
            let parsed: W<T> = toml::from_str(&s).expect("re-parse");
            assert_eq!(parsed.v, w.v);
        }

        check(Format::Mp4, "\"MP4\"");
        check(Format::WebM, "\"WebM\"");
        check(Format::Mp3, "\"MP3\"");
        check(Quality::P1080, "\"1080p\"");
        check(Quality::P2160, "\"2160p\"");
        check(Codec::H264, "\"H.264\"");
        check(Codec::Vp9, "\"VP9\"");
        check(AudioBitrate::K256, "\"256 kbps\"");
        check(AudioBitrate::K96, "\"96 kbps\"");
        check(MaxSize::NoLimit, "\"No limit\"");
        check(MaxSize::Gb1, "\"1 GB\"");
        check(Protocol::Auto, "\"Auto\"");
        check(Protocol::Hls, "\"HLS\"");
        check(Theme::Light, "\"Light\"");
        check(Theme::System, "\"System\"");
    }
}
