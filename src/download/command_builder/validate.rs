//! Build-time validation errors and predicates for the command builder.

use std::fmt;

/// Errors returned by [`super::builder::CommandBuilder::build`].
///
/// Each variant maps to one of the conflict rules listed in the task spec.
/// Conflicts the type system already prevents (audio + video container,
/// multiple cookie sources, recode + remux) cannot be produced through the
/// builder API and therefore have no error variant here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// No URL and no URL-list file supplied.
    NoInput,
    /// `playlist_items` was set together with `playlist_start` or
    /// `playlist_end`. yt-dlp ignores the latter when the former is present,
    /// which is almost always a mistake.
    PlaylistItemsConflictsStartEnd,
    /// Output template references a variable not in the whitelist.
    InvalidTemplateVar { var: String },
    /// Output template has malformed `%(name)s` syntax.
    InvalidTemplateSyntax(String),
    /// `--audio-quality` value is out of the 0..=9 range yt-dlp accepts.
    AudioQualityOutOfRange(u16),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoInput => write!(f, "command builder: no input URL or file given"),
            Self::PlaylistItemsConflictsStartEnd => write!(
                f,
                "command builder: playlist_items cannot be combined with playlist_start/playlist_end"
            ),
            Self::InvalidTemplateVar { var } => write!(
                f,
                "command builder: output template uses disallowed variable %({var})s"
            ),
            Self::InvalidTemplateSyntax(msg) => {
                write!(f, "command builder: invalid template syntax: {msg}")
            }
            Self::AudioQualityOutOfRange(v) => write!(
                f,
                "command builder: audio_quality {v} out of range (must be 0..=9)"
            ),
        }
    }
}

impl std::error::Error for BuildError {}
