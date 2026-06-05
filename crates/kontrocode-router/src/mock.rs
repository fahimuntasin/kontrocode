//! Mock provider for offline development and tests.
//!
//! Simulates streaming by emitting one chunk every ~30ms, computing a
//! deterministic but plausible response based on the request content.
//! This lets us exercise the full agent loop without a real API key.

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::{self, Stream, StreamExt};
use kontrocode_core::{
    CompletionRequest, CompletionResponse, FinishReason, Message, MessageId, ModelId, ModelSpec,
    Result, Role, StreamChunk, Usage,
};

use super::Provider;

/// A no-cost, no-network provider useful for local dev and tests.
///
/// Echoes the user's last message in a structured form, with a small
/// artificial latency to simulate real model output.
#[derive(Debug, Default, Clone)]
pub struct MockProvider {
    /// Per-chunk delay. Tests override this to keep CI fast.
    pub chunk_delay: Duration,
    /// Models this mock exposes.
    pub specs: Vec<ModelSpec>,
}

impl MockProvider {
    /// Construct a default mock with sensible model specs.
    pub fn new() -> Self {
        Self::with_delay(Duration::from_millis(30))
    }

    /// Construct a mock with a specific per-chunk delay.
    pub fn with_delay(delay: Duration) -> Self {
        Self {
            chunk_delay: delay,
            specs: vec![ModelSpec {
                id: ModelId::new("mock", "echo"),
                display_name: "Mock Echo".into(),
                context_window: 128_000,
                max_output_tokens: 8_192,
                input_price_per_mtok: 0.0,
                output_price_per_mtok: 0.0,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: false,
            }],
        }
    }

    /// Construct a fast mock suitable for unit tests.
    pub fn fast() -> Self {
        Self::with_delay(Duration::from_millis(0))
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn id(&self) -> &'static str {
        "mock"
    }

    fn models(&self) -> &[ModelSpec] {
        &self.specs
    }

    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse> {
        let content = synthesize(&req);
        let model = req.model.clone();
        let usage = Usage {
            input_tokens: estimate_tokens(&req.messages),
            output_tokens: estimate_str_tokens(&content),
            cost_usd: 0.0,
        };
        Ok(CompletionResponse {
            model,
            message: Message {
                id: MessageId::new(),
                role: Role::Assistant,
                content,
                tool_calls: Vec::new(),
                tool_call_id: None,
                meta: None,
            },
            finish_reason: FinishReason::Stop,
            usage,
        })
    }

    async fn stream(
        &self,
        req: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let content = synthesize(&req);
        let model = req.model.clone();
        let delay = self.chunk_delay;
        let chunks: Vec<String> = chunk_string(&content, 12);
        let total = chunks.len();
        let input_tokens = estimate_tokens(&req.messages);
        let output_tokens = estimate_str_tokens(&content);

        let stream = stream::iter(chunks.into_iter().enumerate().map(move |(i, delta)| {
            let is_last = i + 1 == total;
            Ok(StreamChunk {
                model: model.clone(),
                delta,
                tool_calls: Vec::new(),
                finish_reason: if is_last {
                    Some(FinishReason::Stop)
                } else {
                    None
                },
                usage: if is_last {
                    Some(Usage {
                        input_tokens,
                        output_tokens,
                        cost_usd: 0.0,
                    })
                } else {
                    None
                },
            })
        }))
        .then(move |chunk_res| async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            chunk_res
        });

        Ok(Box::pin(stream))
    }
}

fn synthesize(req: &CompletionRequest) -> String {
    let last_user = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User)
        .map(|m| m.content.as_str())
        .unwrap_or("(no user message)");

    let tool_names: Vec<&str> = req.tools.iter().map(|t| t.name.as_str()).collect();

    if !tool_names.is_empty() {
        format!(
            "[mock] I would normally use one of {} to help with: {last_user}\n\n\
             (Configure a real provider in `~/.config/kontrocode/config.toml`.)",
            tool_names.join(", ")
        )
    } else {
        format!(
            "[mock response]\n\n\
             You said: \"{last_user}\"\n\n\
             This is the offline mock provider. Connect a real LLM in Phase 2."
        )
    }
}

fn chunk_string(s: &str, size: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in s.chars() {
        buf.push(c);
        if buf.chars().count() >= size {
            out.push(std::mem::take(&mut buf));
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn estimate_tokens(messages: &[Message]) -> u32 {
    messages
        .iter()
        .map(|m| (m.content.len() as u32) / 4 + 4)
        .sum()
}

fn estimate_str_tokens(s: &str) -> u32 {
    (s.len() as u32 / 4) + 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use kontrocode_core::{Message, ToolDefinition};
    use std::time::Instant;

    #[tokio::test]
    async fn mock_complete_echoes_user_message() {
        let p = MockProvider::fast();
        let req = CompletionRequest::new(
            ModelId::new("mock", "echo"),
            vec![Message::user("hello there")],
        );
        let resp = p.complete(req).await.unwrap();
        assert!(resp.message.content.contains("hello there"));
        assert_eq!(resp.finish_reason, FinishReason::Stop);
    }

    #[tokio::test]
    async fn mock_stream_emits_chunks_then_finish() {
        let p = MockProvider::fast();
        let req = CompletionRequest::new(
            ModelId::new("mock", "echo"),
            vec![Message::user("stream me")],
        );
        let mut s = p.stream(req).await.unwrap();
        let mut total_delta = String::new();
        let mut last_finish = None;
        while let Some(chunk) = s.next().await {
            let chunk = chunk.unwrap();
            total_delta.push_str(&chunk.delta);
            if chunk.finish_reason.is_some() {
                last_finish = chunk.finish_reason;
            }
        }
        assert!(total_delta.contains("stream me"));
        assert_eq!(last_finish, Some(FinishReason::Stop));
    }

    #[tokio::test]
    async fn mock_stream_with_tools_mentions_them() {
        let p = MockProvider::fast();
        let mut req = CompletionRequest::new(
            ModelId::new("mock", "echo"),
            vec![Message::user("read a file")],
        );
        req.tools.push(ToolDefinition::new(
            "file_read",
            "Read a file",
            serde_json::json!({}),
        ));
        let resp = p.complete(req).await.unwrap();
        assert!(resp.message.content.contains("file_read"));
    }

    #[tokio::test]
    async fn chunk_delay_actually_delays() {
        let p = MockProvider::with_delay(Duration::from_millis(20));
        let req = CompletionRequest::new(
            ModelId::new("mock", "echo"),
            vec![Message::user("delay me")],
        );
        let started = Instant::now();
        let mut s = p.stream(req).await.unwrap();
        while s.next().await.is_some() {}
        let elapsed = started.elapsed();
        // At least 3 chunks * 20ms = 60ms minimum, allow generous slack.
        assert!(
            elapsed >= Duration::from_millis(50),
            "elapsed = {elapsed:?}"
        );
    }
}
