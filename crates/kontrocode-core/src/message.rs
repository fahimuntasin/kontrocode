//! Agent ↔ model message types.
//!
//! These are the canonical messages exchanged between the agent loop and
//! any LLM provider. They intentionally mirror the OpenAI Chat Completions
//! shape (the de-facto industry standard) so provider adapters are
//! mechanical translations, not architectural rewrites.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a single message in a conversation thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub Uuid);

impl MessageId {
    /// Create a new random message id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Who produced a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The end user.
    User,
    /// The KontroCode agent.
    Assistant,
    /// A tool's output, fed back into the model.
    Tool,
    /// System instructions for the model.
    System,
}

/// A single message in the agent's conversation thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Stable id for this message.
    pub id: MessageId,
    /// Who produced the message.
    pub role: Role,
    /// The textual content. May be empty if the message is tool-only.
    pub content: String,
    /// Tool calls the assistant wants to make (assistant-only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// For tool-role messages: which call this is responding to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Provider-specific metadata (latency, tokens, model). Not sent back
    /// to the model; preserved for the UI and audit log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MessageMeta>,
}

impl Message {
    /// Construct a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            role: Role::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            meta: None,
        }
    }

    /// Construct an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            role: Role::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            meta: None,
        }
    }

    /// Construct a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            role: Role::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            meta: None,
        }
    }

    /// Construct a tool-result message that responds to `tool_call_id`.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            role: Role::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
            meta: None,
        }
    }

    /// Add a tool call to this assistant message.
    pub fn with_tool_call(mut self, call: ToolCall) -> Self {
        self.tool_calls.push(call);
        self
    }
}

/// A tool call the assistant wants the agent to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-assigned id. Must be echoed back in the matching
    /// [`Message::tool_result`].
    pub id: String,
    /// Tool name. Matches a key in the tool registry.
    pub name: String,
    /// Arguments as a JSON object. Providers differ on whether they
    /// return JSON-encoded strings or raw objects; we always normalize
    /// to a `serde_json::Value`.
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Construct a new tool call.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }
}

/// A tool's execution result, returned by the agent loop and forwarded
/// back to the model as a `tool`-role message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The id of the originating [`ToolCall`].
    pub tool_call_id: String,
    /// The tool's name.
    pub tool_name: String,
    /// The tool's output.
    pub output: ToolOutput,
    /// Whether the tool succeeded.
    pub success: bool,
}

/// Tool output: either a string or structured JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolOutput {
    /// Plain-text output.
    Text(String),
    /// Structured JSON output.
    Json(serde_json::Value),
}

impl ToolOutput {
    /// Render as a string for inclusion in a tool-role message.
    pub fn as_message_string(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Json(v) => serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()),
        }
    }
}

impl From<String> for ToolOutput {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<serde_json::Value> for ToolOutput {
    fn from(v: serde_json::Value) -> Self {
        Self::Json(v)
    }
}

/// Per-message metadata, preserved for the UI and audit log but not sent
/// back to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMeta {
    /// Which model produced this message.
    pub model: String,
    /// Which provider served the request.
    pub provider: String,
    /// Tokens consumed.
    pub tokens_in: u32,
    /// Tokens generated.
    pub tokens_out: u32,
    /// Wall-clock latency in milliseconds.
    pub latency_ms: u64,
    /// Cost in USD.
    pub cost_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_role_serialization_uses_lowercase() {
        let json = serde_json::to_string(&Role::Assistant).unwrap();
        assert_eq!(json, "\"assistant\"");
    }

    #[test]
    fn user_message_constructs() {
        let m = Message::user("hi");
        assert_eq!(m.role, Role::User);
        assert_eq!(m.content, "hi");
        assert!(m.tool_calls.is_empty());
    }

    #[test]
    fn tool_result_message_carries_call_id() {
        let m = Message::tool_result("call_123", "file contents here");
        assert_eq!(m.role, Role::Tool);
        assert_eq!(m.tool_call_id.as_deref(), Some("call_123"));
    }

    #[test]
    fn tool_output_renders_json_pretty() {
        let out = ToolOutput::Json(serde_json::json!({"ok": true}));
        let s = out.as_message_string();
        assert!(s.contains("\"ok\""));
        assert!(s.contains("true"));
    }

    #[test]
    fn message_round_trips_through_json() {
        let m = Message::assistant("ok").with_tool_call(ToolCall::new(
            "c1",
            "file_read",
            serde_json::json!({"path": "/x"}),
        ));
        let s = serde_json::to_string(&m).unwrap();
        let back: Message = serde_json::from_str(&s).unwrap();
        assert_eq!(back.tool_calls.len(), 1);
        assert_eq!(back.tool_calls[0].name, "file_read");
    }
}
