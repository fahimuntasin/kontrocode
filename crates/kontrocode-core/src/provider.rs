//! LLM provider contract.
//!
//! All 9 LLM providers implement the [`Provider`](crate::provider) trait
//! indirectly by producing [`CompletionRequest`] and consuming
//! [`CompletionResponse`] / [`StreamChunk`]. This module defines those
//! shared types.
//!
//! Provider-specific extras (Anthropic's `system` block, OpenAI's
//! `response_format`, Google's `safetySettings`) are passed through a
//! `serde_json::Value` bag and translated by each provider adapter.

use serde::{Deserialize, Serialize};

/// Identifies a specific model a provider can serve.
///
/// Format: `"<provider>/<model>"`, e.g. `"anthropic/claude-sonnet-4"`,
/// `"deepseek/deepseek-chat"`. The slash is the canonical separator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelId(pub String);

impl ModelId {
    /// Construct a `ModelId` from a provider and a model name.
    pub fn new(provider: &str, model: &str) -> Self {
        Self(format!("{provider}/{model}"))
    }

    /// Provider portion (before the first `/`).
    pub fn provider(&self) -> &str {
        self.0.split_once('/').map(|(p, _)| p).unwrap_or("")
    }

    /// Model portion (after the first `/`).
    pub fn model(&self) -> &str {
        self.0.split_once('/').map(|(_, m)| m).unwrap_or(&self.0)
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Static description of a model — pricing, context window, capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Canonical model id.
    pub id: ModelId,
    /// Human-readable display name.
    pub display_name: String,
    /// Maximum context window in tokens.
    pub context_window: u32,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// USD per 1M input tokens.
    pub input_price_per_mtok: f64,
    /// USD per 1M output tokens.
    pub output_price_per_mtok: f64,
    /// Supports tool/function calling.
    pub supports_tools: bool,
    /// Supports streaming responses.
    pub supports_streaming: bool,
    /// Supports vision/image inputs.
    pub supports_vision: bool,
}

/// A request to an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The model to call.
    pub model: ModelId,
    /// Full conversation thread.
    pub messages: Vec<crate::Message>,
    /// Tools the model may invoke. Empty vec means "no tools".
    pub tools: Vec<ToolDefinition>,
    /// Sampling temperature. `None` means provider default.
    pub temperature: Option<f32>,
    /// Nucleus sampling. `None` means provider default.
    pub top_p: Option<f32>,
    /// Maximum tokens to generate. `None` means provider default / max.
    pub max_tokens: Option<u32>,
    /// Stop sequences. Provider-specific behavior on duplicates.
    pub stop: Vec<String>,
    /// Provider-specific passthrough. Do not use for portable behavior.
    #[serde(default)]
    pub extra: serde_json::Value,
}

impl CompletionRequest {
    /// Construct a minimal request: model + messages.
    pub fn new(model: ModelId, messages: Vec<crate::Message>) -> Self {
        Self {
            model,
            messages,
            tools: Vec::new(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            stop: Vec::new(),
            extra: serde_json::Value::Null,
        }
    }

    /// Add a tool definition.
    pub fn with_tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set sampling temperature.
    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }

    /// Set max output tokens.
    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }
}

/// A non-streaming response from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Which model produced the response (may differ from request if
    /// fallback occurred).
    pub model: ModelId,
    /// The assistant message.
    pub message: crate::Message,
    /// Why generation stopped.
    pub finish_reason: FinishReason,
    /// Token usage as reported by the provider.
    pub usage: Usage,
}

/// Reason the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural end of turn or explicit `stop:` sequence.
    Stop,
    /// Hit `max_tokens` cap.
    Length,
    /// Model wants to call a tool.
    ToolCalls,
    /// Provider-side content filter.
    ContentFilter,
    /// Provider-specific error; see the response `message` for details.
    Error,
    /// Unknown reason.
    Other,
}

/// Token and cost accounting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed.
    pub input_tokens: u32,
    /// Output tokens generated.
    pub output_tokens: u32,
    /// Cost in USD, computed by the router using the model spec.
    pub cost_usd: f64,
}

impl Usage {
    /// Compute cost given input and output token counts and a model spec.
    pub fn compute(input: u32, output: u32, spec: &ModelSpec) -> Self {
        let cost = (input as f64 / 1_000_000.0) * spec.input_price_per_mtok
            + (output as f64 / 1_000_000.0) * spec.output_price_per_mtok;
        Self {
            input_tokens: input,
            output_tokens: output,
            cost_usd: cost,
        }
    }
}

/// A single chunk from a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Which model is streaming.
    pub model: ModelId,
    /// Incremental text. Empty for tool-call-only chunks.
    pub delta: String,
    /// Incremental tool calls (some providers stream tool calls
    /// argument-by-argument; we aggregate by `id` in the router).
    #[serde(default)]
    pub tool_calls: Vec<crate::ToolCall>,
    /// Set on the final chunk of a stream.
    pub finish_reason: Option<FinishReason>,
    /// Updated usage stats. Provider-dependent: some only report on the
    /// final chunk; some report incrementally.
    pub usage: Option<Usage>,
}

/// Tool definition exposed to the model (OpenAI / Anthropic style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name. Must be unique within a request.
    pub name: String,
    /// Human-readable description for the model.
    pub description: String,
    /// JSON Schema for the tool's parameters.
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Construct a tool definition with a JSON Schema object.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_id_splits_correctly() {
        let m = ModelId::new("anthropic", "claude-sonnet-4");
        assert_eq!(m.provider(), "anthropic");
        assert_eq!(m.model(), "claude-sonnet-4");
        assert_eq!(m.to_string(), "anthropic/claude-sonnet-4");
    }

    #[test]
    fn usage_cost_calculation() {
        let spec = ModelSpec {
            id: ModelId::new("test", "m"),
            display_name: "Test".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            input_price_per_mtok: 3.0,
            output_price_per_mtok: 15.0,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: false,
        };
        let u = Usage::compute(1_000_000, 500_000, &spec);
        assert!((u.cost_usd - 10.5).abs() < 1e-9);
    }

    #[test]
    fn completion_request_builder() {
        let req = CompletionRequest::new(ModelId::new("a", "b"), vec![])
            .with_temperature(0.2)
            .with_max_tokens(1024)
            .with_tool(ToolDefinition::new("x", "do x", serde_json::json!({})));
        assert_eq!(req.temperature, Some(0.2));
        assert_eq!(req.max_tokens, Some(1024));
        assert_eq!(req.tools.len(), 1);
    }
}
