//! The [`Provider`] trait — every LLM backend implements this.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use kontrocode_core::{CompletionRequest, CompletionResponse, ModelSpec, Result, StreamChunk};

/// A backend capable of serving completion requests for one or more models.
///
/// Implementations are required to be `Send + Sync` so the router can hold
/// them behind an `Arc` and share across async tasks.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stable, lowercase identifier (e.g. `"anthropic"`, `"mock"`).
    /// Must match the prefix used in [`ModelId`](kontrocode_core::ModelId).
    fn id(&self) -> &'static str;

    /// All models this provider can serve.
    fn models(&self) -> &[ModelSpec];

    /// Look up a model spec by short model name (the part after the
    /// provider prefix in [`ModelId`](kontrocode_core::ModelId)). Returns
    /// `None` if not served here.
    fn lookup(&self, model_name: &str) -> Option<&ModelSpec> {
        self.models().iter().find(|m| m.id.model() == model_name)
    }

    /// Send a non-streaming completion request.
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;

    /// Send a streaming completion request.
    ///
    /// Implementations that don't support streaming should return
    /// [`kontrocode_core::Error::Provider`] with a descriptive message.
    async fn stream(
        &self,
        req: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;

    /// Whether this provider is currently healthy. The router checks
    /// this before routing; unhealthy providers are skipped.
    async fn healthy(&self) -> bool {
        true
    }

    /// Shut down any held connections / background tasks. Default no-op.
    async fn shutdown(&self) {}
}
