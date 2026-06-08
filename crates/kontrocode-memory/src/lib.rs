//! # kontrocode-memory
//!
//! The memory system. Phase 1 ships a file-backed [`MemoryStore`]
//! implementation. Phase 4 swaps in a Redis-backed implementation
//! behind the same trait.
//!
//! See PRD §4 for the full design.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod decay;
pub mod file_store;
pub mod redis_store;
pub mod signal;
pub mod store;

pub use decay::apply_decay;
pub use file_store::FileMemoryStore;
pub use signal::{Signal, SignalKind};
pub use store::MemoryStore;
