#![allow(dead_code)] // public API consumed by downstream tickets (#13/#23/#25)

pub mod parser;
pub mod types;
pub mod youtube;

// Re-exports for ergonomic imports downstream. Marked
// `#[allow(unused_imports)]` because no in-tree consumer uses them yet
// (tickets #13/#23/#25 are still open).
#[allow(unused_imports)]
pub use types::{VideoId, YouTubeVideo};
#[allow(unused_imports)]
pub use youtube::YouTubeClient;
