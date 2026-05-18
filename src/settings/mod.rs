//! Persistent application settings.
//!
//! `model` defines the typed `AppSettings` struct (and its enums); `persistence`
//! handles loading/saving as TOML under `crate::paths::config_dir()`.
//! Persistence is `pub(crate)` — only `AppService` calls into it.

pub mod model;
pub(crate) mod persistence;

pub use model::*;
