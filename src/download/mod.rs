//! Download orchestration.
//!
//! `command_builder` translates an `AppSettings` snapshot into the argument
//! vector for invoking the bundled `yt-dlp` binary. Sibling modules (spawn,
//! progress parsing, worker, manager) will land in tickets #20-#22.

// Downstream consumers (#20 spawner, #21 worker) have not been written yet,
// so the public surface here is only used by tests for now. Suppress the
// `dead_code` lint at the module level until those callers land.
#![allow(dead_code)]

pub mod command_builder;
pub mod manager;
pub mod progress;
pub mod worker;
pub mod yt_dlp;
