//! Build the argument vector handed to `yt-dlp`.
//!
//! Given an `AppSettings` snapshot, a YouTube video id, and the temp directory
//! that yt-dlp should write into, [`build`] returns a `Vec<OsString>` to be
//! passed verbatim to `Command::new(yt_dlp_path).args(...)`. The URL is always
//! the LAST positional argument so that downstream wrappers can locate it.
//!
//! The two main code paths are:
//! - **Audio-only** (`Format::is_audio_only()`): emits `-x --audio-format ...
//!   --audio-quality <kbps>` and skips the `-f`/`--merge-output-format` flags.
//! - **Video**: emits `-f "bv*[height<=H][vcodec~=RE]+ba/b" --merge-output-format <c>`.
//!
//! Other concerns (max-size cap, extras toggles, auth token) are layered on top
//! of either path. Protocol selection beyond `Auto` is deferred to V2 because
//! it requires per-extractor flags that yt-dlp does not expose generically.

use std::ffi::OsString;
use std::path::Path;

use crate::settings::{AppSettings, AudioBitrate, Codec, Format, MaxSize, Protocol, Quality};

/// Build the full yt-dlp argument vector.
///
/// `temp_dir` is where yt-dlp writes the intermediate file; the worker
/// (#21) moves it to `settings.download_path` once the download is done.
/// The URL is appended last so consumers can do simple positional checks.
pub fn build(settings: &AppSettings, video_id: &str, temp_dir: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();

    // --- Output template -------------------------------------------------
    //
    // Build `<temp_dir>/%(id)s.%(ext)s` via `OsString` join so paths with
    // non-UTF-8 bytes survive on every supported OS.
    args.push(OsString::from("-o"));
    let mut out_template = OsString::from(temp_dir.as_os_str());
    // `Path::join` would interpret the `%(...)s` fragment as a relative path
    // segment; that is fine and matches yt-dlp's expectation. We do it
    // manually here to avoid pulling in PathBuf just to immediately convert.
    let sep = if cfg!(windows) { "\\" } else { "/" };
    out_template.push(sep);
    out_template.push("%(id)s.%(ext)s");
    args.push(out_template);

    // --- Stdout shape ---------------------------------------------------
    //
    // `--newline` forces line-buffered progress lines so the spawner can
    // parse them as JSON one at a time.
    args.push(OsString::from("--newline"));
    args.push(OsString::from("--progress-template"));
    args.push(OsString::from(
        // The `download:` prefix scopes the template to download events
        // (yt-dlp also fires `postprocess:` events otherwise).
        r#"download:{"d":%(progress.downloaded_bytes)s,"t":%(progress.total_bytes)s,"s":"%(progress.status)s","p":"%(progress.elapsed)s"}"#,
    ));
    args.push(OsString::from("--no-warnings"));

    // --- Format selection ------------------------------------------------
    if settings.format.is_audio_only() {
        args.push(OsString::from("-x"));
        args.push(OsString::from("--audio-format"));
        args.push(OsString::from(audio_format_str(&settings.format)));
        args.push(OsString::from("--audio-quality"));
        args.push(OsString::from(audio_bitrate_kbps(&settings.audio_bitrate)));
    } else {
        args.push(OsString::from("-f"));
        args.push(OsString::from(format_selector(
            &settings.quality,
            &settings.codec,
        )));
        args.push(OsString::from("--merge-output-format"));
        args.push(OsString::from(video_container_str(&settings.format)));
    }

    // --- Size cap --------------------------------------------------------
    if let Some(cap) = max_filesize_arg(&settings.max_size) {
        args.push(OsString::from("--max-filesize"));
        args.push(OsString::from(cap));
    }

    // --- Protocol --------------------------------------------------------
    //
    // Only `Auto` is honoured in V1: the other variants require per-extractor
    // flags that yt-dlp does not expose generically. `Auto` is also the
    // default (no flag), so this is a no-op for now — kept here as a hook
    // for V2 with an explicit comment so the pattern match stays exhaustive.
    match settings.protocol {
        Protocol::Auto | Protocol::Https | Protocol::Http | Protocol::Hls | Protocol::Dash => {
            // protocol selection beyond Auto is deferred to V2.
        }
    }

    // --- Extras (each flag-shaped, no values) ----------------------------
    let e = &settings.extras;
    if e.embed_thumbnail {
        args.push(OsString::from("--embed-thumbnail"));
    }
    if e.embed_metadata {
        args.push(OsString::from("--embed-metadata"));
    }
    if e.embed_chapters {
        args.push(OsString::from("--embed-chapters"));
    }
    if e.embed_subtitles {
        args.push(OsString::from("--embed-subs"));
    }
    if e.write_subtitles {
        args.push(OsString::from("--write-subs"));
    }
    if e.skip_playlists {
        args.push(OsString::from("--no-playlist"));
    }
    if e.restrict_names {
        args.push(OsString::from("--restrict-filenames"));
    }

    // --- Auth token ------------------------------------------------------
    //
    // Per the plan ("klärung in V1"), we forward a non-empty token as
    // `--username oauth2 --password <token>`; yt-dlp accepts this for some
    // OAuth-flavoured auth flows. Empty token → omit.
    if !settings.auth_token.is_empty() {
        args.push(OsString::from("--username"));
        args.push(OsString::from("oauth2"));
        args.push(OsString::from("--password"));
        args.push(OsString::from(&settings.auth_token));
    }

    // --- URL (always last) ----------------------------------------------
    args.push(OsString::from(format!(
        "https://www.youtube.com/watch?v={video_id}"
    )));

    args
}

/// Returns the lowercase audio format token expected by `--audio-format`.
fn audio_format_str(f: &Format) -> &'static str {
    match f {
        Format::Mp3 => "mp3",
        Format::M4a => "m4a",
        Format::Aac => "aac",
        Format::Alac => "alac",
        Format::Aiff => "aiff",
        Format::Flac => "flac",
        // The video formats never reach here (caller checks `is_audio_only`),
        // but we still return a sensible string to keep the signature total.
        Format::Mp4 | Format::Mkv | Format::WebM | Format::Mov => "mp3",
    }
}

/// Returns the lowercase container token expected by `--merge-output-format`.
fn video_container_str(f: &Format) -> &'static str {
    match f {
        Format::Mp4 => "mp4",
        Format::Mkv => "mkv",
        Format::WebM => "webm",
        Format::Mov => "mov",
        // Audio formats never reach here; default to mp4 for totality.
        _ => "mp4",
    }
}

/// `AudioBitrate::K256` → `"256"`. yt-dlp expects bare digits for `--audio-quality`.
fn audio_bitrate_kbps(b: &AudioBitrate) -> &'static str {
    match b {
        AudioBitrate::K96 => "96",
        AudioBitrate::K128 => "128",
        AudioBitrate::K192 => "192",
        AudioBitrate::K256 => "256",
        AudioBitrate::K320 => "320",
    }
}

/// Build the `-f` selector string for the video path.
///
/// `bv*[height<=H][vcodec~=RE]+ba/b` — best video matching the height + codec
/// merged with best audio, fallback to single-file `b` if the merge target is
/// unavailable.
fn format_selector(q: &Quality, c: &Codec) -> String {
    let h = match q {
        Quality::P360 => 360,
        Quality::P720 => 720,
        Quality::P1080 => 1080,
        Quality::P1440 => 1440,
        Quality::P2160 => 2160,
    };
    let codec_re = match c {
        Codec::H264 => "^avc1",
        Codec::H265 => "^hev1|^hvc1",
        Codec::Vp9 => "^vp09",
        Codec::Av1 => "^av01",
    };
    format!("bv*[height<={h}][vcodec~={codec_re}]+ba/b")
}

/// Returns the `--max-filesize` value for the given `MaxSize`, or `None` when
/// the user picked "No limit".
fn max_filesize_arg(m: &MaxSize) -> Option<&'static str> {
    match m {
        MaxSize::NoLimit => None,
        MaxSize::Mb50 => Some("50M"),
        MaxSize::Mb100 => Some("100M"),
        MaxSize::Mb500 => Some("500M"),
        MaxSize::Gb1 => Some("1G"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{AppSettings, AudioBitrate, Codec, Extras, Format, MaxSize, Quality};
    use std::path::PathBuf;

    /// Flatten a `Vec<OsString>` to `Vec<String>` via lossy conversion. Tests
    /// only assert on ASCII fragments so loss is not a concern.
    fn flat(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect()
    }

    /// Returns `true` if the slice contains a contiguous subsequence equal to `needle`.
    fn contains_seq(haystack: &[String], needle: &[&str]) -> bool {
        if needle.is_empty() {
            return true;
        }
        haystack
            .windows(needle.len())
            .any(|w| w.iter().zip(needle.iter()).all(|(a, b)| a == b))
    }

    fn base_settings() -> AppSettings {
        // Deliberately diverge from `AppSettings::default()` for the
        // download_path so we never depend on the user's real Downloads dir.
        AppSettings {
            download_path: PathBuf::from("/tmp/dl"),
            // Defaults toggle two extras on; tests that care override this.
            ..AppSettings::default()
        }
    }

    #[test]
    fn default_video_settings_emit_correct_args() {
        let s = base_settings();
        let args = build(&s, "VIDEO_ID", Path::new("/tmp/foo"));
        let f = flat(&args);

        // Format selector is present.
        assert!(contains_seq(&f, &["-f"]), "args: {f:?}");
        // Default codec is H.264 → `^avc1`, default quality 1080.
        assert!(
            f.iter()
                .any(|s| s.contains("bv*[height<=1080][vcodec~=^avc1]+ba/b")),
            "args: {f:?}"
        );
        // Container default = MP4.
        assert!(
            contains_seq(&f, &["--merge-output-format", "mp4"]),
            "args: {f:?}"
        );
        // Output template anchored at the temp dir.
        assert!(contains_seq(&f, &["-o"]), "args: {f:?}");
        assert!(
            f.iter()
                .any(|s| s.contains("/tmp/foo") && s.contains("%(id)s.%(ext)s")),
            "args: {f:?}"
        );
        // Common tracing flags.
        assert!(f.iter().any(|s| s == "--newline"), "args: {f:?}");
        assert!(f.iter().any(|s| s == "--no-warnings"), "args: {f:?}");
        // URL last.
        assert_eq!(
            f.last().map(String::as_str),
            Some("https://www.youtube.com/watch?v=VIDEO_ID")
        );
    }

    #[test]
    fn mp3_audio_only_takes_audio_path() {
        let mut s = base_settings();
        s.format = Format::Mp3;
        s.audio_bitrate = AudioBitrate::K256;
        let args = build(&s, "abc", Path::new("/tmp/foo"));
        let f = flat(&args);

        assert!(
            !f.iter().any(|s| s == "-f"),
            "audio path must omit -f: {f:?}"
        );
        assert!(
            !f.iter().any(|s| s == "--merge-output-format"),
            "audio path must omit --merge-output-format: {f:?}"
        );
        assert!(f.iter().any(|s| s == "-x"), "expected -x: {f:?}");
        assert!(contains_seq(&f, &["--audio-format", "mp3"]), "args: {f:?}");
        assert!(contains_seq(&f, &["--audio-quality", "256"]), "args: {f:?}");
    }

    #[test]
    fn flac_audio_only_overrides_codec_quality() {
        let mut s = base_settings();
        s.format = Format::Flac;
        s.audio_bitrate = AudioBitrate::K320;
        // Codec/quality must NOT influence the audio path.
        s.codec = Codec::Av1;
        s.quality = Quality::P2160;
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);

        assert!(contains_seq(&f, &["--audio-format", "flac"]), "args: {f:?}");
        assert!(contains_seq(&f, &["--audio-quality", "320"]), "args: {f:?}");
        assert!(!f.iter().any(|s| s.contains("bv*")), "args: {f:?}");
    }

    #[test]
    fn extras_emit_corresponding_flags() {
        let mut s = base_settings();
        s.extras = Extras {
            embed_thumbnail: true,
            embed_metadata: true,
            embed_chapters: true,
            embed_subtitles: true,
            write_subtitles: true,
            skip_playlists: true,
            restrict_names: true,
        };
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);

        for flag in [
            "--embed-thumbnail",
            "--embed-metadata",
            "--embed-chapters",
            "--embed-subs",
            "--write-subs",
            "--no-playlist",
            "--restrict-filenames",
        ] {
            assert!(f.iter().any(|s| s == flag), "missing {flag} in {f:?}");
        }
    }

    #[test]
    fn extras_off_omit_flags() {
        let mut s = base_settings();
        s.extras = Extras::default(); // all false
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);

        for flag in [
            "--embed-thumbnail",
            "--embed-metadata",
            "--embed-chapters",
            "--embed-subs",
            "--write-subs",
            "--no-playlist",
            "--restrict-filenames",
        ] {
            assert!(!f.iter().any(|s| s == flag), "unexpected {flag} in {f:?}");
        }
    }

    #[test]
    fn auth_token_emits_oauth2_creds() {
        let mut s = base_settings();
        s.auth_token = "abc".into();
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);

        assert!(contains_seq(&f, &["--username", "oauth2"]), "args: {f:?}");
        assert!(contains_seq(&f, &["--password", "abc"]), "args: {f:?}");
    }

    #[test]
    fn auth_token_empty_omits_creds() {
        let s = base_settings(); // empty token by default
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);

        assert!(!f.iter().any(|s| s == "--username"), "args: {f:?}");
        assert!(!f.iter().any(|s| s == "--password"), "args: {f:?}");
    }

    #[test]
    fn max_size_50mb_emits_flag() {
        let mut s = base_settings();
        s.max_size = MaxSize::Mb50;
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);
        assert!(contains_seq(&f, &["--max-filesize", "50M"]), "args: {f:?}");
    }

    #[test]
    fn max_size_no_limit_omits_flag() {
        let mut s = base_settings();
        s.max_size = MaxSize::NoLimit;
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);
        assert!(!f.iter().any(|s| s == "--max-filesize"), "args: {f:?}");
    }

    #[test]
    fn output_template_uses_temp_dir() {
        let unique = Path::new("/tmp/yt-dlp-gui-uniquetestpath-xyz");
        let s = base_settings();
        let args = build(&s, "id", unique);
        let f = flat(&args);

        // Find the `-o` flag and inspect the immediately following argument.
        let pos = f.iter().position(|s| s == "-o").expect("missing -o flag");
        let template = f.get(pos + 1).expect("missing -o value");
        assert!(
            template.contains("/tmp/yt-dlp-gui-uniquetestpath-xyz"),
            "expected unique temp dir in -o template: {template}"
        );
        assert!(
            template.ends_with("%(id)s.%(ext)s"),
            "template missing yt-dlp placeholders: {template}"
        );
    }

    #[test]
    fn url_is_last_arg() {
        let s = base_settings();
        let args = build(&s, "DEADBEEF42", Path::new("/tmp/foo"));
        let f = flat(&args);
        assert_eq!(
            f.last().map(String::as_str),
            Some("https://www.youtube.com/watch?v=DEADBEEF42")
        );
    }

    #[test]
    fn codec_av1_emits_av01_regex() {
        let mut s = base_settings();
        s.codec = Codec::Av1;
        s.quality = Quality::P720;
        let args = build(&s, "id", Path::new("/tmp/foo"));
        let f = flat(&args);
        assert!(
            f.iter()
                .any(|s| s.contains("bv*[height<=720][vcodec~=^av01]+ba/b")),
            "args: {f:?}"
        );
    }
}
