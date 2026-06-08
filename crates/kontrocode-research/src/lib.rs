//! # kontrocode-research
//!
//! The research agent's parallel fetcher. Phase 1 ships trait definitions
//! and stub implementations. Real fetchers (pub.dev, npm, GitHub, …)
//! land in Phase 3.
//!
//! The runner queries every applicable source concurrently with
//! `tokio::join!` and returns a [`DecisionReport`].
//!
//! See PRD §5.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod cache;
pub mod runner;
pub mod sources;

pub use cache::InMemoryCache;
pub use runner::{ResearchRunner, ResearchRunnerConfig};
pub use sources::{NullSource, ResearchSource};
pub mod version_resolver;
pub mod feed;
