//! Fluent builder for yt-dlp commands.
//!
//! Construct via [`CommandBuilder::new`] (manual) or
//! [`CommandBuilder::from_settings`] (driven by the persisted `AppSettings`),
//! chain section-specific setters, and finalise with [`CommandBuilder::build`]
//! which validates conflicts and emits a [`Command`].
//!
//! See [`super::types`] for the domain model and [`super::validate`] for the
//! conflict rules.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::settings::{AppSettings, AudioBitrate, Codec, Format as SF, MaxSize, Quality};

use super::command::Command;
use super::template::validate_template;
use super::types::*;
use super::validate::BuildError;

/// JSON shape expected by [`crate::download::yt_dlp::spawn_and_track`].
/// Keep byte-identical with the parser there.
const PROGRESS_TEMPLATE: &str = r#"download:{"d":%(progress.downloaded_bytes)s,"t":%(progress.total_bytes)s,"s":"%(progress.status)s","p":"%(progress.elapsed)s"}"#;

/// Fluent builder. Cheap to clone (all fields are owned but small).
#[derive(Debug, Clone)]
pub struct CommandBuilder {
    // --- Input ---
    urls: Vec<String>,
    url_list_file: Option<PathBuf>,

    // --- Format ---
    format: Option<Format>,

    // --- Output ---
    output_template: Option<String>,
    output_path: Option<PathBuf>,
    no_overwrites: bool,
    restrict_filenames: bool,
    trim_filenames: Option<u32>,

    // --- Playlist ---
    playlist_mode: PlaylistMode,
    playlist_start: Option<u32>,
    playlist_end: Option<u32>,
    playlist_items: Option<String>,
    max_downloads: Option<u32>,
    match_title: Option<String>,
    reject_title: Option<String>,
    date: Option<String>,
    datebefore: Option<String>,
    dateafter: Option<String>,
    min_views: Option<u64>,

    // --- Network ---
    rate_limit: Option<String>,
    concurrent_fragments: Option<u32>,
    retries: Option<u32>,
    fragment_retries: Option<u32>,
    cookies: Cookies,
    proxy: Option<String>,
    source_address: Option<String>,
    geo_bypass: bool,

    // --- Metadata ---
    add_metadata: bool,
    embed_metadata: bool,
    write_thumbnail: bool,
    embed_thumbnail: bool,
    write_info_json: bool,
    write_description: bool,

    // --- Subtitles ---
    write_subs: bool,
    write_auto_subs: bool,
    embed_subs: bool,
    sub_langs: Vec<String>,
    sub_format: Option<String>,

    // --- Post-Processing ---
    sponsorblock: SponsorBlock,
    split_chapters: bool,
    embed_chapters: bool,
    download_sections: Option<String>,
    exec: Option<String>,
    ffmpeg_location: Option<PathBuf>,
    postprocessor_args: Option<String>,
    recode_or_remux: RecodeOrRemux,
    merge_output_format_override: Option<String>,

    // --- Auth (carried over from the legacy builder) ---
    auth_token: Option<String>,

    // --- Misc ---
    simulate: bool,
    verbose: bool,
    quiet: bool,
    no_warnings: bool,
    sleep_interval: Option<u32>,

    // --- Transport-level toggles consumed by yt_dlp.rs ---
    progress_json: bool,
    newline: bool,

    // --- File-size cap (legacy `MaxSize` mapping) ---
    max_filesize: Option<String>,

    // --- Custom format selector when `Format::Video` should be overridden ---
    format_selector_override: Option<String>,

    // --- yt-dlp binary used by the consumer (only stored, not emitted) ---
    yt_dlp_binary: Option<PathBuf>,

    // --- Skip-playlists legacy extra (`--no-playlist`) ---
    no_playlist: bool,
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self {
            urls: Vec::new(),
            url_list_file: None,
            format: None,
            output_template: None,
            output_path: None,
            no_overwrites: false,
            restrict_filenames: false,
            trim_filenames: None,
            playlist_mode: PlaylistMode::Single,
            playlist_start: None,
            playlist_end: None,
            playlist_items: None,
            max_downloads: None,
            match_title: None,
            reject_title: None,
            date: None,
            datebefore: None,
            dateafter: None,
            min_views: None,
            rate_limit: None,
            concurrent_fragments: None,
            retries: None,
            fragment_retries: None,
            cookies: Cookies::None,
            proxy: None,
            source_address: None,
            geo_bypass: false,
            add_metadata: false,
            embed_metadata: false,
            write_thumbnail: false,
            embed_thumbnail: false,
            write_info_json: false,
            write_description: false,
            write_subs: false,
            write_auto_subs: false,
            embed_subs: false,
            sub_langs: Vec::new(),
            sub_format: None,
            sponsorblock: SponsorBlock::None,
            split_chapters: false,
            embed_chapters: false,
            download_sections: None,
            exec: None,
            ffmpeg_location: None,
            postprocessor_args: None,
            recode_or_remux: RecodeOrRemux::None,
            merge_output_format_override: None,
            auth_token: None,
            simulate: false,
            verbose: false,
            quiet: false,
            no_warnings: true, // legacy parity: --no-warnings was always on
            sleep_interval: None,
            progress_json: true,
            newline: true,
            max_filesize: None,
            format_selector_override: None,
            yt_dlp_binary: None,
            no_playlist: false,
        }
    }
}

impl CommandBuilder {
    /// Create an empty builder. Use [`Self::from_settings`] to seed it from
    /// the persisted `AppSettings`.
    pub fn new(url: impl Into<String>) -> Self {
        let mut b = Self::default();
        b.urls.push(url.into());
        b
    }

    /// Seed from the persisted application settings, mirroring the behaviour
    /// of the legacy `command_builder::build` function for the
    /// `(AppSettings, video_id, temp_dir)` triple.
    ///
    /// `video_id` is rendered into the canonical YouTube watch URL.
    /// `temp_dir` is prefixed to the `%(title)s [%(id)s].%(ext)s` output
    /// template — preserving the existing on-disk filename layout.
    pub fn from_settings(s: &AppSettings, video_id: &str, temp_dir: &Path) -> Self {
        let mut b = Self::default();

        b.urls
            .push(format!("https://www.youtube.com/watch?v={video_id}"));

        // Output template: temp_dir + canonical placeholder string.
        let sep = if cfg!(windows) { "\\" } else { "/" };
        let mut tpl = temp_dir.to_string_lossy().into_owned();
        tpl.push_str(sep);
        tpl.push_str("%(title)s [%(id)s].%(ext)s");
        b.output_template = Some(tpl);

        // Format / quality / codec → typed Format.
        if s.format.is_audio_only() {
            let container = match s.format {
                SF::Mp3 => AudioContainer::Mp3,
                SF::M4a => AudioContainer::M4a,
                SF::Aac => AudioContainer::Aac,
                SF::Alac => AudioContainer::Alac,
                SF::Aiff => AudioContainer::Aiff,
                SF::Flac => AudioContainer::Flac,
                _ => AudioContainer::Mp3,
            };
            // Map AudioBitrate → yt-dlp 0..=9 audio-quality. The legacy
            // builder emitted kbps literally; yt-dlp accepts both forms, but
            // for parity we forward the kbps value through `Custom`-style by
            // tagging it as a numeric quality.
            //
            // Concretely: yt-dlp accepts `--audio-quality <kbps>` too, so we
            // pick the kbps mapping below and stash it as a non-0..=9
            // "raw" override stored in `format_selector_override`-like
            // companion field. Keep it simple: use the kbps integer directly
            // and let the renderer accept >9.
            let kbps = match s.audio_bitrate {
                AudioBitrate::K96 => 96,
                AudioBitrate::K128 => 128,
                AudioBitrate::K192 => 192,
                AudioBitrate::K256 => 256,
                AudioBitrate::K320 => 320,
            };
            b.format = Some(Format::Audio {
                container,
                quality: kbps,
            });
        } else {
            let container = match s.format {
                SF::Mp4 => VideoContainer::Mp4,
                SF::Mkv => VideoContainer::Mkv,
                SF::WebM => VideoContainer::WebM,
                // `Mov` is not a `--merge-output-format` token; emit it via
                // `merge_output_format_override` and keep VideoContainer at
                // Mp4 as a safe default for the selector.
                SF::Mov => {
                    b.merge_output_format_override = Some("mov".into());
                    VideoContainer::Mp4
                }
                _ => VideoContainer::Mp4,
            };
            let quality = match s.quality {
                Quality::P360 => VideoQuality::Custom("360".into()),
                Quality::P720 => VideoQuality::P720,
                Quality::P1080 => VideoQuality::P1080,
                Quality::P1440 => VideoQuality::Custom("1440".into()),
                Quality::P2160 => VideoQuality::Custom("2160".into()),
            };
            b.format = Some(Format::Video { quality, container });

            // Custom `-f` selector encoding the codec preference, mirroring
            // the legacy `bv*[height<=H][vcodec~=RE]+ba/b` layout.
            let h = match s.quality {
                Quality::P360 => 360,
                Quality::P720 => 720,
                Quality::P1080 => 1080,
                Quality::P1440 => 1440,
                Quality::P2160 => 2160,
            };
            // yt-dlp's `~=` regex filter requires the pattern to be quoted in
            // the format string (e.g. `vcodec~="^avc1"`); without the quotes
            // yt-dlp raises `Invalid filter specification` and the download
            // never starts.
            let codec_re = match s.codec {
                Codec::H264 => r#""^avc1""#,
                Codec::H265 => r#""^(hev1|hvc1)""#,
                Codec::Vp9 => r#""^vp09""#,
                Codec::Av1 => r#""^av01""#,
            };
            b.format_selector_override = Some(format!("bv*[height<={h}][vcodec~={codec_re}]+ba/b"));
        }

        // Legacy `MaxSize` → `--max-filesize`.
        b.max_filesize = match s.max_size {
            MaxSize::NoLimit => None,
            MaxSize::Mb50 => Some("50M".into()),
            MaxSize::Mb100 => Some("100M".into()),
            MaxSize::Mb500 => Some("500M".into()),
            MaxSize::Gb1 => Some("1G".into()),
        };

        // Legacy extras.
        b.embed_thumbnail = s.extras.embed_thumbnail;
        b.embed_metadata = s.extras.embed_metadata;
        b.embed_chapters = s.extras.embed_chapters;
        b.embed_subs = s.extras.embed_subtitles;
        b.write_subs = s.extras.write_subtitles;
        b.no_playlist = s.extras.skip_playlists;
        b.restrict_filenames = s.extras.restrict_names;

        // Auth token → oauth2/password.
        if !s.auth_token.is_empty() {
            b.auth_token = Some(s.auth_token.clone());
        }

        // New sub-structs (defaults when not in TOML).
        let p = &s.playlist;
        b.playlist_mode = p.mode.clone().into();
        b.playlist_start = p.playlist_start;
        b.playlist_end = p.playlist_end;
        b.playlist_items = p.playlist_items.clone();
        b.max_downloads = p.max_downloads;
        b.match_title = p.match_title.clone();
        b.reject_title = p.reject_title.clone();
        b.date = p.date.clone();
        b.datebefore = p.datebefore.clone();
        b.dateafter = p.dateafter.clone();
        b.min_views = p.min_views;

        let n = &s.network;
        b.rate_limit = n.rate_limit.clone();
        b.concurrent_fragments = n.concurrent_fragments;
        b.retries = n.retries;
        b.fragment_retries = n.fragment_retries;
        b.cookies = n.cookies.to_cookies();
        b.proxy = n.proxy.clone();
        b.source_address = n.source_address.clone();
        b.geo_bypass = n.geo_bypass;

        let m = &s.metadata_extras;
        b.add_metadata = m.add_metadata;
        b.write_thumbnail = m.write_thumbnail;
        b.write_info_json = m.write_info_json;
        b.write_description = m.write_description;

        let sub = &s.subtitles;
        b.write_auto_subs = sub.write_auto_subs;
        b.sub_langs = sub.sub_langs.clone();
        b.sub_format = sub.sub_format.clone();

        let pp = &s.post_processing;
        b.sponsorblock = pp.sponsorblock.to_sponsorblock();
        b.split_chapters = pp.split_chapters;
        b.download_sections = pp.download_sections.clone();
        b.exec = pp.exec.clone();
        b.ffmpeg_location = pp.ffmpeg_location.clone();
        b.postprocessor_args = pp.postprocessor_args.clone();
        b.recode_or_remux = pp.recode_or_remux.to_recode_or_remux();

        let misc = &s.misc;
        b.simulate = misc.simulate;
        b.verbose = misc.verbose;
        b.quiet = misc.quiet;
        if misc.no_warnings {
            b.no_warnings = true;
        }
        b.sleep_interval = misc.sleep_interval;

        b
    }

    // === Section 1: Input ===

    pub fn add_url(mut self, url: impl Into<String>) -> Self {
        self.urls.push(url.into());
        self
    }

    pub fn url_list_file(mut self, path: PathBuf) -> Self {
        self.url_list_file = Some(path);
        self
    }

    // === Section 2: Format ===

    pub fn format(mut self, f: Format) -> Self {
        self.format = Some(f);
        self
    }

    /// Override the rendered `-f` selector. Useful for codec-specific
    /// preferences that `Format::Video` does not encode directly.
    pub fn format_selector(mut self, sel: impl Into<String>) -> Self {
        self.format_selector_override = Some(sel.into());
        self
    }

    // === Section 3: Output ===

    pub fn output_template(mut self, tpl: impl Into<String>) -> Self {
        self.output_template = Some(tpl.into());
        self
    }

    pub fn output_path(mut self, p: PathBuf) -> Self {
        self.output_path = Some(p);
        self
    }

    pub fn no_overwrites(mut self) -> Self {
        self.no_overwrites = true;
        self
    }

    pub fn restrict_filenames(mut self) -> Self {
        self.restrict_filenames = true;
        self
    }

    pub fn trim_filenames(mut self, n: u32) -> Self {
        self.trim_filenames = Some(n);
        self
    }

    // === Section 4: Playlist ===

    pub fn playlist_mode(mut self, m: PlaylistMode) -> Self {
        self.playlist_mode = m;
        self
    }

    pub fn playlist_start(mut self, n: u32) -> Self {
        self.playlist_start = Some(n);
        self
    }

    pub fn playlist_end(mut self, n: u32) -> Self {
        self.playlist_end = Some(n);
        self
    }

    pub fn playlist_items(mut self, spec: impl Into<String>) -> Self {
        self.playlist_items = Some(spec.into());
        self
    }

    pub fn max_downloads(mut self, n: u32) -> Self {
        self.max_downloads = Some(n);
        self
    }

    pub fn match_title(mut self, re: impl Into<String>) -> Self {
        self.match_title = Some(re.into());
        self
    }

    pub fn reject_title(mut self, re: impl Into<String>) -> Self {
        self.reject_title = Some(re.into());
        self
    }

    pub fn date(mut self, s: impl Into<String>) -> Self {
        self.date = Some(s.into());
        self
    }

    pub fn datebefore(mut self, s: impl Into<String>) -> Self {
        self.datebefore = Some(s.into());
        self
    }

    pub fn dateafter(mut self, s: impl Into<String>) -> Self {
        self.dateafter = Some(s.into());
        self
    }

    pub fn min_views(mut self, n: u64) -> Self {
        self.min_views = Some(n);
        self
    }

    // === Section 5: Network ===

    pub fn rate_limit(mut self, s: impl Into<String>) -> Self {
        self.rate_limit = Some(s.into());
        self
    }

    pub fn concurrent_fragments(mut self, n: u32) -> Self {
        self.concurrent_fragments = Some(n);
        self
    }

    pub fn retries(mut self, n: u32) -> Self {
        self.retries = Some(n);
        self
    }

    pub fn fragment_retries(mut self, n: u32) -> Self {
        self.fragment_retries = Some(n);
        self
    }

    pub fn cookies(mut self, c: Cookies) -> Self {
        self.cookies = c;
        self
    }

    pub fn proxy(mut self, url: impl Into<String>) -> Self {
        self.proxy = Some(url.into());
        self
    }

    pub fn source_address(mut self, addr: impl Into<String>) -> Self {
        self.source_address = Some(addr.into());
        self
    }

    pub fn geo_bypass(mut self) -> Self {
        self.geo_bypass = true;
        self
    }

    // === Section 6: Metadata ===

    pub fn add_metadata(mut self) -> Self {
        self.add_metadata = true;
        self
    }

    pub fn embed_metadata(mut self) -> Self {
        self.embed_metadata = true;
        self
    }

    pub fn write_thumbnail(mut self) -> Self {
        self.write_thumbnail = true;
        self
    }

    pub fn embed_thumbnail(mut self) -> Self {
        self.embed_thumbnail = true;
        self
    }

    pub fn write_info_json(mut self) -> Self {
        self.write_info_json = true;
        self
    }

    pub fn write_description(mut self) -> Self {
        self.write_description = true;
        self
    }

    // === Section 7: Subtitles ===

    pub fn write_subs(mut self) -> Self {
        self.write_subs = true;
        self
    }

    pub fn write_auto_subs(mut self) -> Self {
        self.write_auto_subs = true;
        self
    }

    pub fn embed_subs(mut self) -> Self {
        self.embed_subs = true;
        self
    }

    pub fn sub_langs(mut self, langs: Vec<String>) -> Self {
        self.sub_langs = langs;
        self
    }

    pub fn sub_format(mut self, fmt: impl Into<String>) -> Self {
        self.sub_format = Some(fmt.into());
        self
    }

    // === Section 8: Post-Processing ===

    pub fn sponsorblock(mut self, sb: SponsorBlock) -> Self {
        self.sponsorblock = sb;
        self
    }

    pub fn split_chapters(mut self) -> Self {
        self.split_chapters = true;
        self
    }

    pub fn embed_chapters(mut self) -> Self {
        self.embed_chapters = true;
        self
    }

    pub fn download_sections(mut self, range: impl Into<String>) -> Self {
        self.download_sections = Some(range.into());
        self
    }

    pub fn exec(mut self, cmd: impl Into<String>) -> Self {
        self.exec = Some(cmd.into());
        self
    }

    pub fn ffmpeg_location(mut self, p: PathBuf) -> Self {
        self.ffmpeg_location = Some(p);
        self
    }

    /// Convenience wrapper for callers that have an `Option<&Path>` already.
    pub fn ffmpeg_location_opt(mut self, p: Option<&Path>) -> Self {
        self.ffmpeg_location = p.map(Path::to_path_buf);
        self
    }

    pub fn postprocessor_args(mut self, args: impl Into<String>) -> Self {
        self.postprocessor_args = Some(args.into());
        self
    }

    pub fn recode_or_remux(mut self, r: RecodeOrRemux) -> Self {
        self.recode_or_remux = r;
        self
    }

    // === Auth (legacy parity) ===

    pub fn auth_token(mut self, t: impl Into<String>) -> Self {
        let s = t.into();
        if !s.is_empty() {
            self.auth_token = Some(s);
        }
        self
    }

    // === Section 9: Misc ===

    pub fn simulate(mut self) -> Self {
        self.simulate = true;
        self
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    pub fn no_warnings(mut self, on: bool) -> Self {
        self.no_warnings = on;
        self
    }

    pub fn sleep_interval(mut self, secs: u32) -> Self {
        self.sleep_interval = Some(secs);
        self
    }

    // === Transport toggles (default on, opt-out) ===

    pub fn with_progress_json(mut self, on: bool) -> Self {
        self.progress_json = on;
        self
    }

    pub fn with_newline(mut self, on: bool) -> Self {
        self.newline = on;
        self
    }

    pub fn max_filesize(mut self, cap: impl Into<String>) -> Self {
        self.max_filesize = Some(cap.into());
        self
    }

    pub fn no_playlist(mut self) -> Self {
        self.no_playlist = true;
        self
    }

    pub fn yt_dlp_binary(mut self, p: PathBuf) -> Self {
        self.yt_dlp_binary = Some(p);
        self
    }

    // === Finalize ===

    /// Validate cross-field rules and render the argument vector.
    pub fn build(self) -> Result<Command, BuildError> {
        // Rule: at least one URL or a list file.
        if self.urls.is_empty() && self.url_list_file.is_none() {
            return Err(BuildError::NoInput);
        }

        // Rule: playlist_items conflicts with playlist_start/end.
        if self.playlist_items.is_some()
            && (self.playlist_start.is_some() || self.playlist_end.is_some())
        {
            return Err(BuildError::PlaylistItemsConflictsStartEnd);
        }

        // Rule: audio-quality 0..=9 (only when the *Format::Audio* path is
        // active AND the kbps mapping was not applied via from_settings).
        if let Some(Format::Audio { quality, .. }) = &self.format {
            // Allow either the 0..=9 yt-dlp scale or kbps values (>=32) used
            // by the legacy AppSettings mapping.
            if !(*quality <= 9 || (32..=512).contains(quality)) {
                return Err(BuildError::AudioQualityOutOfRange(*quality));
            }
        }

        // Rule: output template whitelist.
        if let Some(tpl) = &self.output_template {
            // Strip the directory prefix (everything up to the last separator)
            // before validating: only the template-part may contain `%(...)s`.
            let tail = tpl.rsplit_once(['/', '\\']).map(|t| t.1).unwrap_or(tpl);
            validate_template(tail)?;
        }

        // --- Begin emit -------------------------------------------------
        let mut args: Vec<OsString> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut requires_ffmpeg = false;
        let mut ffmpeg_reasons: Vec<&'static str> = Vec::new();

        // Output template ---------------------------------------------------
        if let Some(tpl) = &self.output_template {
            args.push("-o".into());
            args.push(OsString::from(tpl));
        }
        if let Some(p) = &self.output_path {
            args.push("-P".into());
            args.push(p.into());
        }
        if self.no_overwrites {
            args.push("--no-overwrites".into());
        }
        if self.restrict_filenames {
            args.push("--restrict-filenames".into());
        }
        if let Some(n) = self.trim_filenames {
            args.push("--trim-filenames".into());
            args.push(n.to_string().into());
        }

        // Stdout shape -----------------------------------------------------
        if self.newline {
            args.push("--newline".into());
        }
        if self.progress_json {
            args.push("--progress-template".into());
            args.push(PROGRESS_TEMPLATE.into());
        }
        if self.no_warnings {
            args.push("--no-warnings".into());
        }

        // ffmpeg location ---------------------------------------------------
        if let Some(p) = &self.ffmpeg_location {
            args.push("--ffmpeg-location".into());
            args.push(p.into());
        }

        // Format ------------------------------------------------------------
        match &self.format {
            Some(Format::Audio { container, quality }) => {
                requires_ffmpeg = true;
                ffmpeg_reasons.push("audio extraction (-x)");
                args.push("-x".into());
                args.push("--audio-format".into());
                args.push(container.as_str().into());
                args.push("--audio-quality".into());
                args.push(quality.to_string().into());
            }
            Some(Format::Video { quality, container }) => {
                requires_ffmpeg = true;
                ffmpeg_reasons.push("video merge (bv*+ba)");
                args.push("-f".into());
                let selector = self
                    .format_selector_override
                    .clone()
                    .unwrap_or_else(|| default_video_selector(quality));
                args.push(selector.into());
                let merge = self
                    .merge_output_format_override
                    .clone()
                    .unwrap_or_else(|| container.as_str().into());
                args.push("--merge-output-format".into());
                args.push(merge.into());
            }
            Some(Format::Custom(s)) => {
                args.push("-f".into());
                args.push(s.into());
            }
            None => {}
        }

        // Max filesize ------------------------------------------------------
        if let Some(cap) = &self.max_filesize {
            args.push("--max-filesize".into());
            args.push(cap.into());
        }

        // Playlist ----------------------------------------------------------
        match self.playlist_mode {
            PlaylistMode::Single => {}
            PlaylistMode::Playlist => args.push("--yes-playlist".into()),
        }
        if self.no_playlist {
            args.push("--no-playlist".into());
        }
        if let Some(spec) = &self.playlist_items {
            args.push("--playlist-items".into());
            args.push(spec.into());
        } else {
            if let Some(n) = self.playlist_start {
                args.push("--playlist-start".into());
                args.push(n.to_string().into());
            }
            if let Some(n) = self.playlist_end {
                args.push("--playlist-end".into());
                args.push(n.to_string().into());
            }
        }
        if let Some(n) = self.max_downloads {
            args.push("--max-downloads".into());
            args.push(n.to_string().into());
        }
        if let Some(r) = &self.match_title {
            args.push("--match-title".into());
            args.push(r.into());
        }
        if let Some(r) = &self.reject_title {
            args.push("--reject-title".into());
            args.push(r.into());
        }
        if let Some(d) = &self.date {
            args.push("--date".into());
            args.push(d.into());
        }
        if let Some(d) = &self.datebefore {
            args.push("--datebefore".into());
            args.push(d.into());
        }
        if let Some(d) = &self.dateafter {
            args.push("--dateafter".into());
            args.push(d.into());
        }
        if let Some(v) = self.min_views {
            args.push("--min-views".into());
            args.push(v.to_string().into());
        }

        // Network -----------------------------------------------------------
        if let Some(r) = &self.rate_limit {
            args.push("--limit-rate".into());
            args.push(r.into());
        }
        if let Some(n) = self.concurrent_fragments {
            args.push("--concurrent-fragments".into());
            args.push(n.to_string().into());
        }
        if let Some(n) = self.retries {
            args.push("--retries".into());
            args.push(n.to_string().into());
        }
        if let Some(n) = self.fragment_retries {
            args.push("--fragment-retries".into());
            args.push(n.to_string().into());
        }
        match &self.cookies {
            Cookies::None => {}
            Cookies::File(p) => {
                args.push("--cookies".into());
                args.push(p.into());
            }
            Cookies::Browser(b) => {
                args.push("--cookies-from-browser".into());
                args.push(b.as_str().into());
            }
        }
        if let Some(p) = &self.proxy {
            args.push("--proxy".into());
            args.push(p.into());
        }
        if let Some(a) = &self.source_address {
            args.push("--source-address".into());
            args.push(a.into());
        }
        if self.geo_bypass {
            args.push("--geo-bypass".into());
        }

        // Metadata ----------------------------------------------------------
        if self.add_metadata {
            args.push("--add-metadata".into());
        }
        if self.embed_metadata {
            args.push("--embed-metadata".into());
        }
        if self.write_thumbnail {
            args.push("--write-thumbnail".into());
        }
        if self.embed_thumbnail {
            args.push("--embed-thumbnail".into());
            requires_ffmpeg = true;
            ffmpeg_reasons.push("embed thumbnail");
        }
        if self.write_info_json {
            args.push("--write-info-json".into());
        }
        if self.write_description {
            args.push("--write-description".into());
        }

        // Subtitles ---------------------------------------------------------
        if self.write_subs {
            args.push("--write-subs".into());
        }
        if self.write_auto_subs {
            args.push("--write-auto-subs".into());
        }
        if self.embed_subs {
            args.push("--embed-subs".into());
            requires_ffmpeg = true;
            ffmpeg_reasons.push("embed subtitles");
        }
        if !self.sub_langs.is_empty() {
            args.push("--sub-langs".into());
            args.push(self.sub_langs.join(",").into());
        }
        if let Some(f) = &self.sub_format {
            args.push("--sub-format".into());
            args.push(f.into());
        }

        // Post-Processing ---------------------------------------------------
        match &self.sponsorblock {
            SponsorBlock::None => {}
            SponsorBlock::Remove(segs) => {
                args.push("--sponsorblock-remove".into());
                args.push(join_segments(segs).into());
                requires_ffmpeg = true;
                ffmpeg_reasons.push("sponsorblock-remove");
            }
            SponsorBlock::Mark(segs) => {
                args.push("--sponsorblock-mark".into());
                args.push(join_segments(segs).into());
                requires_ffmpeg = true;
                ffmpeg_reasons.push("sponsorblock-mark");
            }
        }
        if self.split_chapters {
            args.push("--split-chapters".into());
            requires_ffmpeg = true;
            ffmpeg_reasons.push("split-chapters");
        }
        if self.embed_chapters {
            args.push("--embed-chapters".into());
        }
        if let Some(r) = &self.download_sections {
            args.push("--download-sections".into());
            args.push(r.into());
            requires_ffmpeg = true;
            ffmpeg_reasons.push("download-sections");
        }
        if let Some(e) = &self.exec {
            args.push("--exec".into());
            args.push(e.into());
        }
        if let Some(pa) = &self.postprocessor_args {
            args.push("--postprocessor-args".into());
            args.push(pa.into());
        }
        match &self.recode_or_remux {
            RecodeOrRemux::None => {}
            RecodeOrRemux::Recode(c) => {
                args.push("--recode-video".into());
                args.push(c.into());
                requires_ffmpeg = true;
                ffmpeg_reasons.push("recode-video");
            }
            RecodeOrRemux::Remux(c) => {
                args.push("--remux-video".into());
                args.push(c.into());
                requires_ffmpeg = true;
                ffmpeg_reasons.push("remux-video");
            }
        }

        // Misc --------------------------------------------------------------
        if self.simulate {
            args.push("--simulate".into());
            warnings.push("simulate mode active: no files will be written".into());
        }
        if self.verbose {
            args.push("--verbose".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if let Some(s) = self.sleep_interval {
            args.push("--sleep-interval".into());
            args.push(s.to_string().into());
        }

        // Auth (legacy oauth2/password mapping) ---------------------------
        if let Some(t) = &self.auth_token {
            args.push("--username".into());
            args.push("oauth2".into());
            args.push("--password".into());
            args.push(t.into());
        }

        // URL-list file ----------------------------------------------------
        if let Some(p) = &self.url_list_file {
            args.push("-a".into());
            args.push(p.into());
        }

        // URLs always last (legacy invariant).
        for u in &self.urls {
            args.push(u.into());
        }

        Ok(Command {
            args,
            requires_ffmpeg,
            ffmpeg_reasons,
            warnings,
            yt_dlp_binary: self.yt_dlp_binary,
        })
    }
}

fn default_video_selector(q: &VideoQuality) -> String {
    let h = match q {
        VideoQuality::Best => return "bv*+ba/b".into(),
        VideoQuality::P1080 => 1080,
        VideoQuality::P720 => 720,
        VideoQuality::P480 => 480,
        VideoQuality::Custom(s) => return format!("bv*[height<={s}]+ba/b"),
    };
    format!("bv*[height<={h}]+ba/b")
}

fn join_segments(segs: &[Segment]) -> String {
    segs.iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(",")
}
