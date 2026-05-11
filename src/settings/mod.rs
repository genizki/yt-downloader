//! Persistent application settings.
//!
//! `model` defines the typed `AppSettings` struct (and its enums); `persistence`
//! handles loading/saving as TOML under `crate::paths::config_dir()`.
//! Re-exports below give callers a flat `crate::settings::{AppSettings, load,
//! save, ...}` API without exposing the module split.

pub mod model;
pub mod persistence;

// Re-exports flatten the `crate::settings::{AppSettings, load, save, ...}`
// surface for downstream consumers (tickets #14 settings UI, #19 command
// builder). Until those land none of these names are referenced from
// outside the module, so the compiler emits `unused_imports` warnings —
// silenced explicitly because the module-level `#![allow(dead_code)]` in
// the submodules does not cover re-exports.
#[allow(unused_imports)]
pub use model::*;
#[allow(unused_imports)]
pub use persistence::{load, save, settings_path};
