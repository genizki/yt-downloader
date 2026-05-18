//! Auto-detect classifier for the search bar input.
//!
//! Maps a raw user string onto one of three [`SearchKind`] variants:
//! * a free-text query,
//! * a YouTube video ID (extracted from a URL or naked),
//! * a YouTube playlist ID (extracted from a URL or naked).
//!
//! See `## YouTube-API-Flow` in the project plan for how the result feeds the
//! API client.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

/// What the user's input was recognised as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum SearchKind {
    /// Free-text search query (already trimmed).
    Query(String),
    /// 11-character YouTube video ID.
    VideoId(String),
    /// YouTube playlist ID (`PL…`, `UU…`, `RD…`, `FL…`, `OL…`, …).
    PlaylistId(String),
}

/// Matches `?list=<ID>` or `&list=<ID>` anywhere in a URL.
///
/// Playlist IDs accept the same character class YouTube uses
/// (`A-Za-z0-9_-`) and a generous length floor; the prefix family is
/// validated downstream when a *naked* ID is checked.
static PLAYLIST_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[?&]list=(?P<id>[A-Za-z0-9_-]{10,})").expect("playlist URL regex compiles")
});

/// Matches `youtube.com/watch?v=<11-char-ID>` (with optional extra params).
static WATCH_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)youtube\.com/watch\?(?:[^ ]*&)?v=(?P<id>[A-Za-z0-9_-]{11})")
        .expect("watch URL regex compiles")
});

/// Matches `youtu.be/<11-char-ID>`.
static SHORT_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)youtu\.be/(?P<id>[A-Za-z0-9_-]{11})").expect("youtu.be URL regex compiles")
});

/// Matches `youtube.com/shorts/<11-char-ID>`.
static SHORTS_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)youtube\.com/shorts/(?P<id>[A-Za-z0-9_-]{11})")
        .expect("shorts URL regex compiles")
});

/// Matches `youtube.com/embed/<11-char-ID>`.
static EMBED_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)youtube\.com/embed/(?P<id>[A-Za-z0-9_-]{11})")
        .expect("embed URL regex compiles")
});

/// Naked playlist ID: known prefix families followed by ≥10 ID-chars.
static NAKED_PLAYLIST_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:PL|UU|FL|RD|OL)[A-Za-z0-9_-]{10,}$").expect("naked playlist regex compiles")
});

/// Naked video ID: exactly 11 ID-chars.
static NAKED_VIDEO_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9_-]{11}$").expect("naked video regex compiles"));

/// Classify the raw user input from the search bar.
///
/// Priority order (per plan, section *YouTube-API-Flow*):
/// 1. trim → empty becomes `Query("")`
/// 2. playlist URL (`?list=` / `&list=`) — preferred over `watch?v=` if both
/// 3. video URL (`watch?v=`, `youtu.be/`, `shorts/`, `embed/`)
/// 4. naked playlist ID
/// 5. naked 11-char video ID
/// 6. fallback: free-text query
pub fn classify(input: &str) -> SearchKind {
    let trimmed = input.trim();

    // 1. empty
    if trimmed.is_empty() {
        return SearchKind::Query(String::new());
    }

    // 2. playlist URL — if URL also has `watch?v=`, playlist still wins.
    if let Some(caps) = PLAYLIST_URL_RE.captures(trimmed) {
        if let Some(id) = caps.name("id") {
            return SearchKind::PlaylistId(id.as_str().to_owned());
        }
    }

    // 3. video URL forms
    for re in [&WATCH_URL_RE, &SHORT_URL_RE, &SHORTS_URL_RE, &EMBED_URL_RE] {
        if let Some(caps) = re.captures(trimmed) {
            if let Some(id) = caps.name("id") {
                return SearchKind::VideoId(id.as_str().to_owned());
            }
        }
    }

    // 4. naked playlist ID
    if NAKED_PLAYLIST_RE.is_match(trimmed) {
        return SearchKind::PlaylistId(trimmed.to_owned());
    }

    // 5. naked 11-char video ID
    if NAKED_VIDEO_RE.is_match(trimmed) {
        return SearchKind::VideoId(trimmed.to_owned());
    }

    // 6. fallback
    SearchKind::Query(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_text_query_passes_through() {
        assert_eq!(
            classify("how submarines work"),
            SearchKind::Query("how submarines work".to_owned())
        );
    }

    #[test]
    fn whitespace_is_trimmed_in_query() {
        assert_eq!(
            classify("  lofi study mix  "),
            SearchKind::Query("lofi study mix".to_owned())
        );
    }

    #[test]
    fn empty_input_yields_empty_query() {
        assert_eq!(classify(""), SearchKind::Query(String::new()));
        assert_eq!(classify("   "), SearchKind::Query(String::new()));
    }

    #[test]
    fn watch_url_extracts_video_id() {
        assert_eq!(
            classify("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            SearchKind::VideoId("dQw4w9WgXcQ".to_owned())
        );
    }

    #[test]
    fn short_url_extracts_video_id() {
        assert_eq!(
            classify("https://youtu.be/dQw4w9WgXcQ"),
            SearchKind::VideoId("dQw4w9WgXcQ".to_owned())
        );
    }

    #[test]
    fn shorts_url_extracts_video_id() {
        assert_eq!(
            classify("https://www.youtube.com/shorts/abc12345678"),
            SearchKind::VideoId("abc12345678".to_owned())
        );
    }

    #[test]
    fn playlist_url_extracts_playlist_id() {
        assert_eq!(
            classify("https://www.youtube.com/playlist?list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"),
            SearchKind::PlaylistId("PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf".to_owned())
        );
    }

    #[test]
    fn watch_with_list_prefers_playlist() {
        assert_eq!(
            classify("https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLABC1234567890123"),
            SearchKind::PlaylistId("PLABC1234567890123".to_owned())
        );
    }

    #[test]
    fn naked_playlist_id_is_recognised() {
        assert_eq!(
            classify("PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"),
            SearchKind::PlaylistId("PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf".to_owned())
        );
    }

    #[test]
    fn naked_eleven_char_id_is_video() {
        assert_eq!(
            classify("dQw4w9WgXcQ"),
            SearchKind::VideoId("dQw4w9WgXcQ".to_owned())
        );
    }

    #[test]
    fn three_char_input_falls_back_to_query() {
        assert_eq!(classify("abc"), SearchKind::Query("abc".to_owned()));
    }
}
