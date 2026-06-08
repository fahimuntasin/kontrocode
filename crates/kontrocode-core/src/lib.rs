//! # kontrocode-core
//!
//! Shared types, config, and errors for the KontroCode workspace.
//!
//! Every other crate in the workspace depends on this one. The contract
//! between crates lives here: agent ↔ model messages, router ↔ providers,
//! memory store ↔ profile, research runner ↔ sources.
//!
//! If two crates need to talk, define the message type in this crate.
//! Crate-to-crate private types are not allowed to cross workspace
//! boundaries.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod config;
pub mod embedding;
pub mod error;
pub mod intent;
pub mod message;
pub mod profile;
pub mod provider;
pub mod research;

pub use config::Config;
pub use error::{Error, Result};
pub use intent::{analyze, Intent, Stack, TaskType};
pub use message::{Message, MessageId, Role, ToolCall, ToolOutput, ToolResult};
pub use profile::{
    ExpertiseLevel, Fact, FactSource, Interest, Language, Preferences, Profile, ResponseStyle,
    StackConfidence,
};
pub use provider::{
    CompletionRequest, CompletionResponse, FinishReason, ModelId, ModelSpec, StreamChunk,
    ToolDefinition, Usage,
};
pub use research::{DecisionReport, DeprecatedPattern, ResearchCandidate, ResearchSource};

/// Re-exported version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
