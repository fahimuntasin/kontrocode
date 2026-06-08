//! # kontrocode-agent
//!
//! The agent loop. The brain of KontroCode. Implements the OpenCode-style
//! observe → plan → act loop with tool use, self-correction, research,
//! and memory injection.
//!
//! Phase 1: a minimal but real loop with file_read, file_write, shell_run
//! tools, intent analysis, research, and memory. The model is the
//! [`MockProvider`](kontrocode_router::MockProvider). Real providers
//! land in Phase 2.
//!
//! See PRD §11 for the system prompt and §7 for capability requirements.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod acp;
pub mod decide;
pub mod loop_;
pub mod prompt;
pub mod stream;
pub mod tools;

pub use loop_::{Agent, AgentConfig, AgentOutput};
pub use stream::{AgentEvent, StreamSender};
pub mod hooks;
pub mod rules;
pub mod mcp_runtime;
