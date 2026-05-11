//! Runtime-loaded JSON i18n.
//!
//! Translations are read from `assets/i18n/<lang>.json` at runtime so the user
//! can edit them without recompiling. See [`loader`] for the public API.

// The downstream consumers (settings overlay, hero, result rows, …) are still
// being implemented in parallel. Until they land, the i18n public API is not
// invoked from production code paths — only from the unit tests.
#![allow(dead_code)]

pub mod loader;

#[allow(unused_imports)]
pub use loader::*;
