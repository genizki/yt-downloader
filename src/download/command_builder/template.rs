//! Output template validation.
//!
//! yt-dlp accepts a templating mini-language (`%(field)s`, `%(field|fallback)s`,
//! numeric formatting, ...). The task spec mandates a strict whitelist of
//! allowed variable names; only the syntactic shape `%(name)s` is recognised
//! here. Variable names that pass the whitelist are accepted as-is; any other
//! identifier yields `BuildError::InvalidTemplateVar`.

use super::validate::BuildError;

/// Whitelist of yt-dlp template variables permitted by the builder.
///
/// Add to this list rather than disabling the validator. The list mirrors the
/// task spec exactly so future reviewers can match it against the brief.
pub const ALLOWED_VARS: &[&str] = &[
    "title",
    "id",
    "uploader",
    "upload_date",
    "playlist_index",
    "ext",
];

/// Validate a yt-dlp output template against [`ALLOWED_VARS`].
///
/// Recognises `%(name)s` segments. Whitespace inside the parens and modifiers
/// (`%(name|fallback)s`, `%(name)05d`, ...) are not supported by the strict
/// whitelist — adding them would require also widening the allowed-var set.
pub fn validate_template(tpl: &str) -> Result<(), BuildError> {
    let bytes = tpl.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        // `%%` is an escaped literal percent.
        if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
            i += 2;
            continue;
        }
        if i + 1 >= bytes.len() || bytes[i + 1] != b'(' {
            return Err(BuildError::InvalidTemplateSyntax(format!(
                "unexpected '%' at byte offset {i}"
            )));
        }
        let name_start = i + 2;
        // The variable name must be an ASCII identifier
        // (`[A-Za-z_][A-Za-z0-9_]*`). Anything else is a syntax error rather
        // than an unknown variable, so the message points at the actual issue.
        let mut j = name_start;
        while j < bytes.len() {
            let b = bytes[j];
            let id = b.is_ascii_alphanumeric() || b == b'_';
            if !id {
                break;
            }
            j += 1;
        }
        if j == name_start || j >= bytes.len() || bytes[j] != b')' {
            return Err(BuildError::InvalidTemplateSyntax(
                "missing closing ')' in `%(...)s`".to_string(),
            ));
        }
        let name_end = j;
        if name_end + 1 >= bytes.len() || bytes[name_end + 1] != b's' {
            return Err(BuildError::InvalidTemplateSyntax(
                "expected 's' after `%(...)`".to_string(),
            ));
        }
        let name = &tpl[name_start..name_end];
        if !ALLOWED_VARS.contains(&name) {
            return Err(BuildError::InvalidTemplateVar {
                var: name.to_string(),
            });
        }
        i = name_end + 2;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_legacy_template_passes() {
        validate_template("%(title)s [%(id)s].%(ext)s").unwrap();
    }

    #[test]
    fn all_whitelisted_vars_pass() {
        validate_template(
            "%(uploader)s/%(upload_date)s-%(playlist_index)s-%(title)s.%(id)s.%(ext)s",
        )
        .unwrap();
    }

    #[test]
    fn disallowed_var_returns_err() {
        let e = validate_template("%(channel)s.%(ext)s").unwrap_err();
        assert!(matches!(e, BuildError::InvalidTemplateVar { ref var } if var == "channel"));
    }

    #[test]
    fn missing_close_paren_errs() {
        let e = validate_template("%(title.%(ext)s").unwrap_err();
        assert!(matches!(e, BuildError::InvalidTemplateSyntax(_)));
    }

    #[test]
    fn missing_s_suffix_errs() {
        let e = validate_template("%(title)").unwrap_err();
        assert!(matches!(e, BuildError::InvalidTemplateSyntax(_)));
    }

    #[test]
    fn literal_percent_escape_allowed() {
        validate_template("100%%-%(title)s.%(ext)s").unwrap();
    }

    #[test]
    fn no_template_vars_is_ok() {
        validate_template("static-filename.mp4").unwrap();
    }
}
