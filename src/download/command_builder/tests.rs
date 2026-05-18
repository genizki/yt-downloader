//! Behavioural tests for the new builder.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::settings::{AppSettings, AudioBitrate, Codec, Extras, Format as SF, MaxSize, Quality};

use super::builder::CommandBuilder;
use super::types::*;
use super::validate::BuildError;

fn flat(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect()
}

fn contains_seq(haystack: &[String], needle: &[&str]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|w| w.iter().zip(needle.iter()).all(|(a, b)| a == b))
}

fn base_settings() -> AppSettings {
    AppSettings {
        download_path: PathBuf::from("/tmp/dl"),
        ..AppSettings::default()
    }
}

fn build_from(s: &AppSettings, video_id: &str, temp: &Path) -> Vec<OsString> {
    CommandBuilder::from_settings(s, video_id, temp)
        .build()
        .expect("settings-derived build must succeed")
        .into_args()
}

fn public_build_args_from(
    s: &AppSettings,
    video_id: &str,
    temp: &Path,
    ffmpeg_location: Option<&Path>,
) -> Vec<String> {
    super::build_args(s, video_id, temp, ffmpeg_location)
        .expect("settings-derived build must succeed")
}

#[test]
fn default_video_settings_emit_correct_args() {
    let s = base_settings();
    let args = build_from(&s, "VIDEO_ID", Path::new("/tmp/foo"));
    let f = flat(&args);

    assert!(contains_seq(&f, &["-f"]), "args: {f:?}");
    assert!(
        f.iter()
            .any(|s| s.contains(r#"bv*[height<=1080][vcodec~="^avc1"]+ba/b"#)),
        "args: {f:?}"
    );
    assert!(
        contains_seq(&f, &["--merge-output-format", "mp4"]),
        "args: {f:?}"
    );
    assert!(contains_seq(&f, &["-o"]), "args: {f:?}");
    assert!(
        f.iter()
            .any(|s| s.contains("/tmp/foo") && s.contains("%(title)s [%(id)s].%(ext)s")),
        "args: {f:?}"
    );
    assert!(f.iter().any(|s| s == "--newline"));
    assert!(f.iter().any(|s| s == "--no-warnings"));
    assert_eq!(
        f.last().map(String::as_str),
        Some("https://www.youtube.com/watch?v=VIDEO_ID")
    );
}

#[test]
fn public_build_args_matches_builder_output() {
    let mut s = base_settings();
    s.format = SF::Mp3;
    s.audio_bitrate = AudioBitrate::K256;

    let via_public = public_build_args_from(
        &s,
        "VID_123",
        Path::new("/tmp/foo"),
        Some(Path::new("/opt/ffmpeg/bin/ffmpeg")),
    );
    let via_builder = flat(
        &CommandBuilder::from_settings(&s, "VID_123", Path::new("/tmp/foo"))
            .ffmpeg_location_opt(Some(Path::new("/opt/ffmpeg/bin/ffmpeg")))
            .build()
            .expect("builder build must succeed")
            .into_args(),
    );

    assert_eq!(via_public, via_builder);
}

#[test]
fn mp3_audio_only_takes_audio_path() {
    let mut s = base_settings();
    s.format = SF::Mp3;
    s.audio_bitrate = AudioBitrate::K256;
    let args = build_from(&s, "abc", Path::new("/tmp/foo"));
    let f = flat(&args);

    assert!(!f.iter().any(|s| s == "-f"));
    assert!(!f.iter().any(|s| s == "--merge-output-format"));
    assert!(f.iter().any(|s| s == "-x"));
    assert!(contains_seq(&f, &["--audio-format", "mp3"]));
    assert!(contains_seq(&f, &["--audio-quality", "256"]));
}

#[test]
fn flac_audio_only_overrides_codec_quality() {
    let mut s = base_settings();
    s.format = SF::Flac;
    s.audio_bitrate = AudioBitrate::K320;
    s.codec = Codec::Av1;
    s.quality = Quality::P2160;
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
    let f = flat(&args);

    assert!(contains_seq(&f, &["--audio-format", "flac"]));
    assert!(contains_seq(&f, &["--audio-quality", "320"]));
    assert!(!f.iter().any(|s| s.contains("bv*")));
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
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
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
        assert!(f.iter().any(|s| s == flag), "missing {flag}");
    }
}

#[test]
fn extras_off_omit_flags() {
    let mut s = base_settings();
    s.extras = Extras::default();
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
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
        assert!(!f.iter().any(|s| s == flag), "unexpected {flag}");
    }
}

#[test]
fn auth_token_emits_oauth2_creds() {
    let mut s = base_settings();
    s.auth_token = "abc".into();
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
    let f = flat(&args);
    assert!(contains_seq(&f, &["--username", "oauth2"]));
    assert!(contains_seq(&f, &["--password", "abc"]));
}

#[test]
fn auth_token_empty_omits_creds() {
    let s = base_settings();
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
    let f = flat(&args);
    assert!(!f.iter().any(|s| s == "--username"));
    assert!(!f.iter().any(|s| s == "--password"));
}

#[test]
fn max_size_50mb_emits_flag() {
    let mut s = base_settings();
    s.max_size = MaxSize::Mb50;
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
    let f = flat(&args);
    assert!(contains_seq(&f, &["--max-filesize", "50M"]));
}

#[test]
fn max_size_no_limit_omits_flag() {
    let mut s = base_settings();
    s.max_size = MaxSize::NoLimit;
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
    let f = flat(&args);
    assert!(!f.iter().any(|s| s == "--max-filesize"));
}

#[test]
fn output_template_uses_temp_dir() {
    let unique = Path::new("/tmp/yt-dlp-gui-uniquetestpath-xyz");
    let s = base_settings();
    let args = build_from(&s, "id", unique);
    let f = flat(&args);
    let pos = f.iter().position(|s| s == "-o").expect("missing -o");
    let template = f.get(pos + 1).expect("missing -o value");
    assert!(template.contains("/tmp/yt-dlp-gui-uniquetestpath-xyz"));
    assert!(template.ends_with("%(title)s [%(id)s].%(ext)s"));
}

#[test]
fn url_is_last_arg() {
    let s = base_settings();
    let args = build_from(&s, "DEADBEEF42", Path::new("/tmp/foo"));
    let f = flat(&args);
    assert_eq!(
        f.last().map(String::as_str),
        Some("https://www.youtube.com/watch?v=DEADBEEF42")
    );
}

#[test]
fn ffmpeg_path_emits_location_flag() {
    let s = base_settings();
    let ffmpeg = Path::new("/opt/ffmpeg/bin/ffmpeg");
    let cmd = CommandBuilder::from_settings(&s, "id", Path::new("/tmp/foo"))
        .ffmpeg_location_opt(Some(ffmpeg))
        .build()
        .unwrap();
    let f = flat(cmd.args());
    assert!(contains_seq(
        &f,
        &["--ffmpeg-location", "/opt/ffmpeg/bin/ffmpeg"]
    ));
}

#[test]
fn ffmpeg_path_none_omits_location_flag() {
    let s = base_settings();
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
    let f = flat(&args);
    assert!(!f.iter().any(|s| s == "--ffmpeg-location"));
}

#[test]
fn codec_av1_emits_av01_regex() {
    let mut s = base_settings();
    s.codec = Codec::Av1;
    s.quality = Quality::P720;
    let args = build_from(&s, "id", Path::new("/tmp/foo"));
    let f = flat(&args);
    assert!(f
        .iter()
        .any(|s| s.contains(r#"bv*[height<=720][vcodec~="^av01"]+ba/b"#)));
}

#[test]
fn empty_builder_returns_no_input() {
    let e = CommandBuilder::default().build().unwrap_err();
    assert_eq!(e, BuildError::NoInput);
}

#[test]
fn playlist_items_with_start_returns_err() {
    let e = CommandBuilder::new("https://example/x")
        .playlist_items("1-5")
        .playlist_start(2)
        .build()
        .unwrap_err();
    assert_eq!(e, BuildError::PlaylistItemsConflictsStartEnd);
}

#[test]
fn playlist_items_with_end_returns_err() {
    let e = CommandBuilder::new("https://example/x")
        .playlist_items("1-5")
        .playlist_end(2)
        .build()
        .unwrap_err();
    assert_eq!(e, BuildError::PlaylistItemsConflictsStartEnd);
}

#[test]
fn invalid_template_var_returns_err() {
    let e = CommandBuilder::new("https://example/x")
        .output_template("%(channel)s.%(ext)s")
        .build()
        .unwrap_err();
    assert!(matches!(e, BuildError::InvalidTemplateVar { ref var } if var == "channel"));
}

#[test]
fn default_template_passes_validation() {
    CommandBuilder::new("https://example/x")
        .output_template("/tmp/foo/%(title)s [%(id)s].%(ext)s")
        .build()
        .expect("legacy template must remain valid");
}

#[test]
fn audio_quality_out_of_range_errs() {
    let e = CommandBuilder::new("https://example/x")
        .format(Format::Audio {
            container: AudioContainer::Mp3,
            quality: 600,
        })
        .build()
        .unwrap_err();
    assert!(matches!(e, BuildError::AudioQualityOutOfRange(600)));
}

#[test]
fn audio_quality_kbps_192_ok() {
    CommandBuilder::new("https://example/x")
        .format(Format::Audio {
            container: AudioContainer::Mp3,
            quality: 192,
        })
        .build()
        .unwrap();
}

#[test]
fn audio_quality_vbr_5_ok() {
    CommandBuilder::new("https://example/x")
        .format(Format::Audio {
            container: AudioContainer::Mp3,
            quality: 5,
        })
        .build()
        .unwrap();
}

#[test]
fn embed_subs_marks_requires_ffmpeg() {
    let cmd = CommandBuilder::new("https://example/x")
        .embed_subs()
        .build()
        .unwrap();
    assert!(cmd.requires_ffmpeg());
    assert!(cmd.ffmpeg_reasons().contains(&"embed subtitles"));
}

#[test]
fn sponsorblock_remove_marks_requires_ffmpeg() {
    let cmd = CommandBuilder::new("https://example/x")
        .sponsorblock(SponsorBlock::Remove(vec![Segment::Sponsor]))
        .build()
        .unwrap();
    assert!(cmd.requires_ffmpeg());
    assert!(cmd.ffmpeg_reasons().contains(&"sponsorblock-remove"));
}

#[test]
fn audio_format_marks_requires_ffmpeg() {
    let cmd = CommandBuilder::new("https://example/x")
        .format(Format::Audio {
            container: AudioContainer::Mp3,
            quality: 192,
        })
        .build()
        .unwrap();
    assert!(cmd.requires_ffmpeg());
    assert!(cmd.ffmpeg_reasons().contains(&"audio extraction (-x)"));
}

#[test]
fn plain_url_only_no_ffmpeg_needed() {
    let cmd = CommandBuilder::new("https://example/x").build().unwrap();
    assert!(!cmd.requires_ffmpeg());
    assert!(cmd.ffmpeg_reasons().is_empty());
}

#[test]
fn cookies_file_emits_flag() {
    let cmd = CommandBuilder::new("https://example/x")
        .cookies(Cookies::File(PathBuf::from("/tmp/c.txt")))
        .build()
        .unwrap();
    let f = flat(cmd.args());
    assert!(contains_seq(&f, &["--cookies", "/tmp/c.txt"]));
}

#[test]
fn cookies_browser_emits_flag() {
    let cmd = CommandBuilder::new("https://example/x")
        .cookies(Cookies::Browser(BrowserKind::Firefox))
        .build()
        .unwrap();
    let f = flat(cmd.args());
    assert!(contains_seq(&f, &["--cookies-from-browser", "firefox"]));
}

#[test]
fn simulate_emits_flag_and_warning() {
    let cmd = CommandBuilder::new("https://example/x")
        .simulate()
        .build()
        .unwrap();
    let f = flat(cmd.args());
    assert!(f.iter().any(|s| s == "--simulate"));
    assert!(!cmd.warnings().is_empty());
}

#[test]
fn sub_langs_joined_with_commas() {
    let cmd = CommandBuilder::new("https://example/x")
        .write_subs()
        .sub_langs(vec!["en".into(), "de".into(), "fr".into()])
        .build()
        .unwrap();
    let f = flat(cmd.args());
    assert!(contains_seq(&f, &["--sub-langs", "en,de,fr"]));
}

#[test]
fn recode_emits_flag() {
    let cmd = CommandBuilder::new("https://example/x")
        .recode_or_remux(RecodeOrRemux::Recode("mp4".into()))
        .build()
        .unwrap();
    let f = flat(cmd.args());
    assert!(contains_seq(&f, &["--recode-video", "mp4"]));
}

#[test]
fn url_list_file_emits_dash_a() {
    let cmd = CommandBuilder::default()
        .url_list_file(PathBuf::from("/tmp/urls.txt"))
        .build()
        .unwrap();
    let f = flat(cmd.args());
    assert!(contains_seq(&f, &["-a", "/tmp/urls.txt"]));
}

#[test]
fn shell_string_renders_binary_and_args() {
    let cmd = CommandBuilder::new("https://example/x with space")
        .build()
        .unwrap()
        .with_binary(PathBuf::from("/usr/bin/yt-dlp"));
    let s = cmd.to_shell_string();
    assert!(s.starts_with("/usr/bin/yt-dlp "));
    assert!(s.contains("'https://example/x with space'"));
}

#[test]
fn into_std_uses_binary() {
    let cmd = CommandBuilder::new("https://example/x")
        .build()
        .unwrap()
        .with_binary(PathBuf::from("/opt/yt-dlp"));
    let std_cmd: std::process::Command = cmd.into();
    assert_eq!(std_cmd.get_program().to_string_lossy(), "/opt/yt-dlp");
}
