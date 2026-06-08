//! # kontrocode-router
//!
//! Multi-provider LLM routing. Every LLM call goes through the [`Router`].
//! The router selects a model based on task complexity, current budget,
//! provider health, and the user's preferred optimization mode, then
//! delegates to a [`Provider`] implementation.
//!
//! Phase 1 ships:
//! - The [`Provider`] trait
//! - A [`MockProvider`] for offline testing
//! - The router with cost-optimized default scoring
//!
//! Real providers (Anthropic, OpenAI, Google, …) land in Phase 2.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod embedding;
pub mod mock;
pub mod provider;
pub mod providers;
pub mod registry;
pub mod router;
pub mod scorer;

pub use mock::MockProvider;
pub use provider::Provider;
pub use registry::ProviderRegistry;
pub use router::{Router, RouterConfig, RouterEvent};
pub use scorer::{ScoredModel, ScoringCriteria, TaskComplexity};
pub mod monitor;
