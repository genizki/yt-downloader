//! JSON-backed i18n loader.
//!
//! Translations live in `assets/i18n/<lang>.json` as a flat
//! `HashMap<String, String>` with dot-namespaced keys. The user is free to
//! edit those files at runtime, so we deliberately read them from disk via
//! [`std::fs`] instead of embedding them with `include_str!`.
//!
//! ## Lookup semantics
//!
//! [`I18n::get`] performs a three-step fallback:
//! 1. the active language's map,
//! 2. the eagerly-loaded English map (always available if `en.json` exists),
//! 3. the key itself (so missing keys are visible to translators).
//!
//! ## Template placeholders
//!
//! Some strings (e.g. `search.results_count_template`) contain `{name}` style
//! placeholders. Substitution is *not* performed by this module — callers
//! resolve the template and replace `{name}` themselves. This keeps the loader
//! free of formatting policy.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use anyhow::{Context, Result};

/// Default fallback language. Always loaded eagerly when available so
/// [`I18n::get`] never performs disk I/O.
const FALLBACK_LANG: &str = "en";

/// Active translation table for a single language.
struct Strings {
    lang: String,
    map: HashMap<String, String>,
}

/// Runtime i18n holder.
///
/// Cheap to construct; switching languages is the only operation that hits
/// disk. The English fallback map is loaded once at construction time and
/// kept around for the lifetime of the [`I18n`].
pub struct I18n {
    /// Currently-active language and its key→value map.
    current: RwLock<Strings>,
    /// English fallback. `None` if `en.json` is missing or invalid at boot.
    fallback: Option<HashMap<String, String>>,
}

impl I18n {
    /// Boot with a starting language.
    ///
    /// On any failure (missing or invalid file), falls back to `en`. If `en`
    /// itself is unavailable, the active map is empty and [`I18n::get`] will
    /// echo each key back to the caller — visible but non-fatal.
    pub fn new(lang: &str) -> Self {
        let fallback = load_lang_file(FALLBACK_LANG).ok();

        let (active_lang, active_map) = match load_lang_file(lang) {
            Ok(map) => (lang.to_string(), map),
            Err(_) if lang != FALLBACK_LANG => match fallback.clone() {
                Some(map) => (FALLBACK_LANG.to_string(), map),
                None => (FALLBACK_LANG.to_string(), HashMap::new()),
            },
            Err(_) => (FALLBACK_LANG.to_string(), HashMap::new()),
        };

        I18n {
            current: RwLock::new(Strings {
                lang: active_lang,
                map: active_map,
            }),
            fallback,
        }
    }

    /// Switch to `lang`, reading `assets/i18n/<lang>.json` from disk.
    ///
    /// Returns `Err` on read or parse failure; the active language stays
    /// untouched in that case so the caller can decide whether to roll back
    /// or surface the error to the user.
    pub fn set_language(&self, lang: &str) -> Result<()> {
        let map = load_lang_file(lang)
            .with_context(|| format!("failed to load i18n bundle for language `{lang}`"))?;

        let mut current = self.current.write().expect("i18n RwLock poisoned (write)");
        current.lang = lang.to_string();
        current.map = map;
        Ok(())
    }

    /// Look up a translation key.
    ///
    /// Falls back to the English bundle if loaded, then to the key itself so
    /// untranslated strings surface visibly during development.
    pub fn get(&self, key: &str) -> String {
        let current = self.current.read().expect("i18n RwLock poisoned (read)");
        if let Some(v) = current.map.get(key) {
            return v.clone();
        }
        if let Some(fallback) = self.fallback.as_ref() {
            if let Some(v) = fallback.get(key) {
                return v.clone();
            }
        }
        key.to_string()
    }

    /// Returns the currently-active language code (e.g. `"en"`, `"de"`).
    pub fn language(&self) -> String {
        self.current
            .read()
            .expect("i18n RwLock poisoned (read)")
            .lang
            .clone()
    }
}

/// Resolve the directory that holds the `<lang>.json` files.
///
/// Two strategies, tried in order:
/// 1. **Dev builds**: `CARGO_MANIFEST_DIR/assets/i18n` if the env var is set
///    *and* the directory exists. This makes `cargo run` / `cargo test` find
///    the files without copying them next to the binary.
/// 2. **Shipped builds**: `<exe-dir>/assets/i18n` — mirrors the layout used
///    by [`crate::paths::yt_dlp_binary_path`] for the bundled binaries.
///
/// Falls back to the exe-relative path even if the executable cannot be
/// resolved, returning a relative `assets/i18n` path. Callers should treat
/// the returned path as advisory and surface a descriptive error if a read
/// against it fails.
pub fn i18n_dir() -> PathBuf {
    if let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest).join("assets").join("i18n");
        if candidate.is_dir() {
            return candidate;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.join("assets").join("i18n");
        }
    }

    PathBuf::from("assets").join("i18n")
}

/// Read and parse `<i18n_dir>/<lang>.json` into a flat string map.
fn load_lang_file(lang: &str) -> Result<HashMap<String, String>> {
    let path = i18n_dir().join(format!("{lang}.json"));
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read i18n file at {}", path.display()))?;
    let map: HashMap<String, String> = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse i18n JSON at {}", path.display()))?;
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i18n_loads_english_by_default() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.language(), "en");
        assert_eq!(i18n.get("search.placeholder"), "Start typing for search");
    }

    #[test]
    fn i18n_switches_to_german() {
        let i18n = I18n::new("en");
        i18n.set_language("de").expect("de bundle must load");
        assert_eq!(i18n.language(), "de");
        assert_eq!(i18n.get("search.placeholder"), "Tippe um zu suchen");
        assert_eq!(i18n.get("search.hero"), "Was möchtest du herunterladen?");
    }

    #[test]
    fn i18n_falls_back_to_english_for_missing_key() {
        // Write a temporary minimal bundle that only contains a single key,
        // then point the loader at that directory via CARGO_MANIFEST_DIR.
        // We instead exercise the in-process fallback by mutating the active
        // map directly — that is the unit under test.
        let i18n = I18n::new("de");
        // `app.title` is identical in both bundles, so use a key that exists
        // in EN but not in DE. To guarantee that condition without editing
        // the shipped JSON, we inject a synthetic missing key by removing it
        // from the active map.
        {
            let mut current = i18n.current.write().unwrap();
            current.map.remove("search.placeholder");
        }
        // The English fallback loaded at construction time still has it.
        assert_eq!(i18n.get("search.placeholder"), "Start typing for search");
    }

    #[test]
    fn i18n_falls_back_to_key_for_unknown_key() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.get("nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn i18n_invalid_lang_errs() {
        let i18n = I18n::new("en");
        let before = i18n.language();
        let result = i18n.set_language("xx");
        assert!(result.is_err(), "set_language must fail for missing bundle");
        assert_eq!(
            i18n.language(),
            before,
            "active language must stay unchanged after a failed switch"
        );
    }
}
