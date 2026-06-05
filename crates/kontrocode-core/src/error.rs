//! Unified error type for the KontroCode workspace.
//!
//! All crates in this workspace return [`Result<T, Error>`]. The boundary
//! between crates and the Tauri shell converts to a human-readable string;
//! the structured error lives here.
//!
//! Use [`anyhow::Error`] only at the outermost binary boundary (CLI, Tauri
//! shell). Library code in this workspace must use this typed `Error`.

use thiserror::Error;

/// Convenience result alias for the KontroCode workspace.
pub type Result<T> = std::result::Result<T, Error>;

/// The single error type used by every KontroCode crate.
#[derive(Debug, Error)]
pub enum Error {
    /// A required configuration value is missing or invalid.
    #[error("configuration error: {0}")]
    Config(String),

    /// The user profile could not be loaded or saved.
    #[error("memory store error: {0}")]
    Memory(String),

    /// A LLM provider returned an error.
    #[error("provider '{provider}' error: {message}")]
    Provider {
        /// The provider identifier (e.g. "anthropic", "openai").
        provider: String,
        /// Human-readable error message.
        message: String,
    },

    /// A research source could not be reached or returned invalid data.
    #[error("research source '{source_id}' error: {message}")]
    Research {
        /// The source identifier (e.g. "pub.dev", "npm").
        source_id: String,
        /// Human-readable error message.
        message: String,
    },

    /// A file system operation failed.
    #[error("file system error: {0}")]
    Filesystem(String),

    /// A shell command execution failed.
    #[error("shell error: {0}")]
    Shell(String),

    /// A tool invocation failed.
    #[error("tool '{tool}' error: {message}")]
    Tool {
        /// The tool name (e.g. "file_read", "shell_run").
        tool: String,
        /// Human-readable error message.
        message: String,
    },

    /// The agent aborted (user cancel or timeout).
    #[error("agent aborted: {0}")]
    Aborted(String),

    /// Serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A network error occurred.
    #[error("network error: {0}")]
    Network(String),

    /// The operation was cancelled.
    #[error("cancelled")]
    Cancelled,

    /// An error that doesn't fit a more specific variant.
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Construct a [`Error::Config`].
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Construct a [`Error::Memory`].
    pub fn memory(msg: impl Into<String>) -> Self {
        Self::Memory(msg.into())
    }

    /// Construct a [`Error::Provider`].
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Construct a [`Error::Research`].
    pub fn research(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Research {
            source_id: source.into(),
            message: message.into(),
        }
    }

    /// Construct a [`Error::Filesystem`].
    pub fn filesystem(msg: impl Into<String>) -> Self {
        Self::Filesystem(msg.into())
    }

    /// Construct a [`Error::Shell`].
    pub fn shell(msg: impl Into<String>) -> Self {
        Self::Shell(msg.into())
    }

    /// Construct a [`Error::Tool`].
    pub fn tool(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Tool {
            tool: tool.into(),
            message: message.into(),
        }
    }

    /// Construct a [`Error::Network`].
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    /// Construct a [`Error::Other`].
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// Returns `true` if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Provider { .. } | Self::Io(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_is_human_readable() {
        let err = Error::provider("anthropic", "rate limited");
        assert_eq!(err.to_string(), "provider 'anthropic' error: rate limited");
    }

    #[test]
    fn error_is_retryable_for_network() {
        assert!(Error::network("timeout").is_retryable());
        assert!(!Error::config("bad").is_retryable());
    }

    #[test]
    fn error_from_serde() {
        let json_err: serde_json::Error = serde_json::from_str::<i32>("not a number").unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Serde(_)));
    }
}
