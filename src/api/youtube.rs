//! YouTube Data API v3 client.
//!
//! Implements the dispatch flow from `## YouTube-API-Flow` in the project
//! plan:
//!
//! * `Query`      → `search.list` → harvest video IDs → `videos.list`
//! * `VideoId`    → `videos.list` directly (single ID)
//! * `PlaylistId` → `playlistItems.list` paginated → harvest IDs → `videos.list`
//!
//! Whatever the input, the public output is always a `Vec<YouTubeVideo>`
//! with `duration_seconds` populated.
#![allow(dead_code)] // consumed by tickets #13/#23/#25

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::api::parser::SearchKind;
use crate::api::types::{VideoId, YouTubeVideo};

/// `videos.list` accepts at most 50 IDs per call.
const VIDEOS_LIST_BATCH: usize = 50;
/// `search.list` page size (the API's max is 50, but the plan calls for 25).
const SEARCH_PAGE_SIZE: u32 = 25;
/// `playlistItems.list` page size.
const PLAYLIST_PAGE_SIZE: u32 = 50;

const SEARCH_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3/search";
const VIDEOS_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3/videos";
const PLAYLIST_ITEMS_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3/playlistItems";

/// Thin async wrapper around the YouTube Data v3 API. Holds a long-lived
/// `reqwest::Client` so connection pooling / TLS state is reused across
/// calls.
#[derive(Debug, Clone)]
pub struct YouTubeClient {
    api_key: String,
    http: reqwest::Client,
}

impl YouTubeClient {
    /// Build a new client with the user-provided API key.
    ///
    /// The key is **not** validated here; an invalid key surfaces as a 400
    /// from the first request, which we propagate as `anyhow::Error`.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Resolve a parsed [`SearchKind`] to a list of fully populated
    /// [`YouTubeVideo`]s.
    pub async fn resolve(&self, kind: &SearchKind) -> Result<Vec<YouTubeVideo>> {
        match kind {
            SearchKind::Query(q) => {
                if q.is_empty() {
                    return Ok(Vec::new());
                }
                let ids = self.search_ids(q).await?;
                if ids.is_empty() {
                    return Ok(Vec::new());
                }
                self.videos_lookup(&ids).await
            }
            SearchKind::VideoId(id) => self.videos_lookup(std::slice::from_ref(id)).await,
            SearchKind::PlaylistId(playlist) => {
                let ids = self.playlist_ids(playlist).await?;
                if ids.is_empty() {
                    return Ok(Vec::new());
                }
                self.videos_lookup(&ids).await
            }
        }
    }

    /// `search.list?part=snippet&type=video&maxResults=25&q=…` → just the
    /// video IDs. We could capture title/channel/etc. opportunistically
    /// but `videos.list` (called immediately after) overwrites those fields
    /// anyway, so we keep the function focused.
    async fn search_ids(&self, query: &str) -> Result<Vec<String>> {
        let url = format!(
            "{SEARCH_ENDPOINT}?part=snippet&type=video&maxResults={SEARCH_PAGE_SIZE}\
             &q={}",
            urlencoded(query)
        );
        tracing::debug!(target: "yt::api", endpoint = "search.list", "GET {SEARCH_ENDPOINT}?part=snippet&type=video&maxResults={SEARCH_PAGE_SIZE}&q={}", urlencoded(query));

        let with_key = format!("{url}&key={}", self.api_key);
        let resp = self
            .http
            .get(&with_key)
            .send()
            .await
            .context("send search.list request")?;
        let status = resp.status();
        let text = resp.text().await.context("read search.list body")?;
        if !status.is_success() {
            bail!("YouTube API search.list returned {status}: {text}");
        }
        let parsed: SearchListResponse =
            serde_json::from_str(&text).context("parse search.list response")?;
        Ok(parsed
            .items
            .into_iter()
            .filter_map(|i| i.id.video_id)
            .collect())
    }

    /// Paginate `playlistItems.list` until `nextPageToken` runs out.
    async fn playlist_ids(&self, playlist_id: &str) -> Result<Vec<String>> {
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut url = format!(
                "{PLAYLIST_ITEMS_ENDPOINT}?part=contentDetails&maxResults={PLAYLIST_PAGE_SIZE}\
                 &playlistId={}",
                urlencoded(playlist_id)
            );
            if let Some(token) = &page_token {
                url.push_str("&pageToken=");
                url.push_str(&urlencoded(token));
            }
            tracing::debug!(target: "yt::api", endpoint = "playlistItems.list", "GET {url}");

            let with_key = format!("{url}&key={}", self.api_key);
            let resp = self
                .http
                .get(&with_key)
                .send()
                .await
                .context("send playlistItems.list request")?;
            let status = resp.status();
            let text = resp.text().await.context("read playlistItems.list body")?;
            if !status.is_success() {
                bail!("YouTube API playlistItems.list returned {status}: {text}");
            }
            let parsed: PlaylistItemsResponse =
                serde_json::from_str(&text).context("parse playlistItems.list response")?;
            for item in parsed.items {
                if let Some(id) = item.content_details.video_id {
                    out.push(id);
                }
            }
            match parsed.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }
        Ok(out)
    }

    /// Chunk video IDs into groups of 50 and call `videos.list` for each
    /// batch sequentially. Returns the merged list, in input order.
    async fn videos_lookup(&self, ids: &[String]) -> Result<Vec<YouTubeVideo>> {
        let mut out = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(VIDEOS_LIST_BATCH) {
            let joined = chunk.join(",");
            let url = format!(
                "{VIDEOS_ENDPOINT}?part=contentDetails,statistics,snippet&id={}",
                urlencoded(&joined)
            );
            tracing::debug!(target: "yt::api", endpoint = "videos.list", "GET {url}");

            let with_key = format!("{url}&key={}", self.api_key);
            let resp = self
                .http
                .get(&with_key)
                .send()
                .await
                .context("send videos.list request")?;
            let status = resp.status();
            let text = resp.text().await.context("read videos.list body")?;
            if !status.is_success() {
                bail!("YouTube API videos.list returned {status}: {text}");
            }
            let parsed: VideosListResponse =
                serde_json::from_str(&text).context("parse videos.list response")?;
            for raw in parsed.items {
                out.push(raw.into_youtube_video());
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// JSON shapes (private, only the subset of fields we actually read).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SearchListResponse {
    #[serde(default)]
    items: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    id: SearchItemId,
}

#[derive(Debug, Deserialize)]
struct SearchItemId {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemsResponse {
    #[serde(default)]
    items: Vec<PlaylistItem>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    #[serde(rename = "contentDetails")]
    content_details: PlaylistItemContentDetails,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemContentDetails {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VideosListResponse {
    #[serde(default)]
    items: Vec<VideosListItem>,
}

#[derive(Debug, Deserialize)]
struct VideosListItem {
    id: String,
    #[serde(default)]
    snippet: Option<VideoSnippet>,
    #[serde(rename = "contentDetails", default)]
    content_details: Option<ContentDetails>,
    #[serde(default)]
    statistics: Option<Statistics>,
}

#[derive(Debug, Deserialize)]
struct VideoSnippet {
    #[serde(default)]
    title: Option<String>,
    #[serde(rename = "channelTitle", default)]
    channel_title: Option<String>,
    #[serde(rename = "publishedAt", default)]
    published_at: Option<String>,
    #[serde(default)]
    thumbnails: Option<Thumbnails>,
}

#[derive(Debug, Deserialize)]
struct Thumbnails {
    #[serde(default)]
    default: Option<Thumbnail>,
    #[serde(default)]
    medium: Option<Thumbnail>,
    #[serde(default)]
    high: Option<Thumbnail>,
    #[serde(default)]
    standard: Option<Thumbnail>,
    #[serde(default)]
    maxres: Option<Thumbnail>,
}

#[derive(Debug, Deserialize)]
struct Thumbnail {
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentDetails {
    #[serde(default)]
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Statistics {
    #[serde(rename = "viewCount", default)]
    view_count: Option<String>,
}

impl VideosListItem {
    /// Project a raw `videos.list` item into the domain `YouTubeVideo`.
    /// Missing/malformed fields fall back to safe defaults rather than
    /// failing the whole request.
    fn into_youtube_video(self) -> YouTubeVideo {
        let snippet = self.snippet.unwrap_or(VideoSnippet {
            title: None,
            channel_title: None,
            published_at: None,
            thumbnails: None,
        });
        let thumbnail_url = pick_thumbnail(&snippet.thumbnails);
        let published_at = snippet
            .published_at
            .as_deref()
            .and_then(parse_rfc3339)
            .unwrap_or_else(epoch_zero);
        let duration_seconds = self
            .content_details
            .as_ref()
            .and_then(|cd| cd.duration.as_deref())
            .map(parse_iso_duration)
            .unwrap_or(0);
        let views = self
            .statistics
            .as_ref()
            .and_then(|s| s.view_count.as_deref())
            .map(parse_view_count)
            .unwrap_or(0);

        YouTubeVideo {
            id: VideoId::new(self.id),
            title: snippet.title.unwrap_or_default(),
            channel: snippet.channel_title.unwrap_or_default(),
            duration_seconds,
            views,
            published_at,
            thumbnail_url,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers.
// ---------------------------------------------------------------------------

/// Parse an ISO-8601 duration of the shape `PT#H#M#S` (the YouTube API only
/// emits the time part for video lengths). Returns total seconds.
///
/// Robust to:
/// * any subset of the H/M/S components (`PT4M13S`, `PT1H30M`, `PT45S`)
/// * empty / `P0D` / non-`PT` inputs (returns `0`)
/// * unparseable numerics anywhere (returns `0`)
///
/// Weeks/days/months/years are intentionally ignored — YouTube video
/// durations max out at ~12 hours in practice.
fn parse_iso_duration(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    // Support `PT…` and `P…T…` shapes; locate the `T` and only scan after.
    let after_t = match s.find('T') {
        Some(i) => &s[i + 1..],
        None => return 0,
    };
    if after_t.is_empty() {
        return 0;
    }

    let mut total: u64 = 0;
    let mut buf = String::new();
    for ch in after_t.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            buf.push(ch);
            continue;
        }
        // `H`/`M`/`S` close out the current numeric run.
        let value: f64 = buf.parse().unwrap_or(0.0);
        buf.clear();
        match ch {
            'H' | 'h' => total = total.saturating_add((value * 3600.0) as u64),
            'M' | 'm' => total = total.saturating_add((value * 60.0) as u64),
            'S' | 's' => total = total.saturating_add(value as u64),
            _ => return 0, // unexpected unit → bail
        }
    }
    total
}

/// Convert a YouTube `viewCount` string into a `u64`. The API ships counts
/// as JSON strings (so they don't overflow JS `Number.MAX_SAFE_INTEGER`);
/// any non-numeric / empty value is treated as `0`.
fn parse_view_count(s: &str) -> u64 {
    s.trim().parse::<u64>().unwrap_or(0)
}

/// Highest-quality thumbnail wins. Order: `maxres` → `standard` → `high` →
/// `medium` → `default`. Returns empty string when no thumbnail is set —
/// the result row deals with empty URLs gracefully.
fn pick_thumbnail(t: &Option<Thumbnails>) -> String {
    let Some(t) = t else { return String::new() };
    for thumb in [&t.maxres, &t.standard, &t.high, &t.medium, &t.default]
        .into_iter()
        .flatten()
    {
        if let Some(url) = &thumb.url {
            if !url.is_empty() {
                return url.clone();
            }
        }
    }
    String::new()
}

/// RFC3339 → `DateTime<Utc>`, returning `None` on parse failure.
fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Sentinel `DateTime<Utc>` used when the API omits/garbles `publishedAt`.
///
/// `from_timestamp(0, 0)` is documented to return `Some(epoch)`; the
/// `unwrap_or_default()` guards the worst-case where chrono ever returns
/// `None` (it would still give us the Unix epoch via `DateTime::default()`).
fn epoch_zero() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default()
}

/// Minimal URL-encoder for query-string values. We only ever pass these
/// through `format!`, never user-controlled keys, so we just escape the
/// reserved set instead of pulling in a third dependency. Spaces become
/// `%20` (not `+`) because that's what `youtube/v3` documents in its
/// examples and what `reqwest` would produce.
fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => {
                out.push('%');
                out.push_str(&format!("{other:02X}"));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use serde_json::json;

    #[test]
    fn iso_duration_parses_minutes_and_seconds() {
        assert_eq!(parse_iso_duration("PT4M13S"), 253);
    }

    #[test]
    fn iso_duration_parses_hours_and_minutes() {
        assert_eq!(parse_iso_duration("PT1H30M"), 5400);
    }

    #[test]
    fn iso_duration_parses_seconds_only() {
        assert_eq!(parse_iso_duration("PT45S"), 45);
    }

    #[test]
    fn iso_duration_handles_empty_and_zero_day() {
        assert_eq!(parse_iso_duration(""), 0);
        // P0D has no T → we treat it as 0 per plan.
        assert_eq!(parse_iso_duration("P0D"), 0);
        assert_eq!(parse_iso_duration("PT0S"), 0);
    }

    #[test]
    fn iso_duration_full_hms() {
        assert_eq!(parse_iso_duration("PT2H15M30S"), 2 * 3600 + 15 * 60 + 30);
    }

    #[test]
    fn view_count_string_parsing() {
        assert_eq!(parse_view_count("1234"), 1234);
        assert_eq!(parse_view_count(""), 0);
        assert_eq!(parse_view_count("abc"), 0);
        assert_eq!(parse_view_count("  42  "), 42);
    }

    #[test]
    fn thumbnail_picks_highest_available() {
        // synth a JSON value with maxres + high + medium + default
        let raw = json!({
            "default": {"url": "low"},
            "medium":  {"url": "med"},
            "high":    {"url": "hi"},
            "standard":{"url": "std"},
            "maxres":  {"url": "max"}
        });
        let t: Thumbnails = serde_json::from_value(raw).unwrap();
        assert_eq!(pick_thumbnail(&Some(t)), "max");

        // No maxres or standard → high wins.
        let raw = json!({
            "default": {"url": "low"},
            "medium":  {"url": "med"},
            "high":    {"url": "hi"}
        });
        let t: Thumbnails = serde_json::from_value(raw).unwrap();
        assert_eq!(pick_thumbnail(&Some(t)), "hi");

        // high over medium
        let raw = json!({
            "medium":  {"url": "med"},
            "high":    {"url": "hi"}
        });
        let t: Thumbnails = serde_json::from_value(raw).unwrap();
        assert_eq!(pick_thumbnail(&Some(t)), "hi");

        // medium over default
        let raw = json!({
            "default": {"url": "low"},
            "medium":  {"url": "med"}
        });
        let t: Thumbnails = serde_json::from_value(raw).unwrap();
        assert_eq!(pick_thumbnail(&Some(t)), "med");

        // only default → default
        let raw = json!({ "default": {"url": "low"} });
        let t: Thumbnails = serde_json::from_value(raw).unwrap();
        assert_eq!(pick_thumbnail(&Some(t)), "low");

        // empty → empty string
        assert_eq!(pick_thumbnail(&None), "");
    }

    #[test]
    fn rfc3339_parses_round_trip() {
        let dt = parse_rfc3339("2024-01-15T10:00:00Z").expect("parse");
        assert_eq!(dt.timestamp(), 1705312800);
    }

    #[test]
    fn epoch_zero_returns_unix_epoch() {
        let e = super::epoch_zero();
        assert_eq!(e.timestamp(), 0);
    }

    #[test]
    fn videos_list_item_projects_to_domain_video() {
        let raw = json!({
            "id": "abc12345678",
            "snippet": {
                "title": "Sample Title",
                "channelTitle": "Sample Channel",
                "publishedAt": "2024-01-15T10:00:00Z",
                "thumbnails": {
                    "high": { "url": "https://example.com/hi.jpg" }
                }
            },
            "contentDetails": { "duration": "PT4M13S" },
            "statistics": { "viewCount": "1234567" }
        });
        let item: VideosListItem = serde_json::from_value(raw).unwrap();
        let v = item.into_youtube_video();
        assert_eq!(v.id.as_str(), "abc12345678");
        assert_eq!(v.title, "Sample Title");
        assert_eq!(v.channel, "Sample Channel");
        assert_eq!(v.duration_seconds, 253);
        assert_eq!(v.duration(), Duration::from_secs(253));
        assert_eq!(v.views, 1_234_567);
        assert_eq!(v.thumbnail_url, "https://example.com/hi.jpg");
        assert_eq!(v.published_at.timestamp(), 1705312800);
    }

    #[test]
    fn videos_list_item_with_missing_fields_is_lenient() {
        let raw = json!({ "id": "abc12345678" });
        let item: VideosListItem = serde_json::from_value(raw).unwrap();
        let v = item.into_youtube_video();
        assert_eq!(v.id.as_str(), "abc12345678");
        assert_eq!(v.title, "");
        assert_eq!(v.channel, "");
        assert_eq!(v.duration_seconds, 0);
        assert_eq!(v.views, 0);
        assert_eq!(v.thumbnail_url, "");
    }

    #[test]
    fn urlencoded_escapes_reserved_chars() {
        assert_eq!(urlencoded("hello world"), "hello%20world");
        assert_eq!(urlencoded("a+b&c=d"), "a%2Bb%26c%3Dd");
        assert_eq!(urlencoded("plain-id_42"), "plain-id_42");
    }
}
