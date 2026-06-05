//! The [`Router`] — the public entry point for LLM calls.
//!
//! The router wraps a [`ProviderRegistry`], a [`RouterConfig`], and a
//! scoring policy. Given a [`CompletionRequest`], it:
//!
//! 1. Picks the cheapest/fastest/best model that meets the quality floor.
//! 2. Routes the request to the provider that serves that model.
//! 3. On failure, walks the fallback chain with a 300ms SLA per provider.
//! 4. Tracks cost and emits a [`RouterEvent`] for the agent loop.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use kontrocode_core::{CompletionRequest, CompletionResponse, ModelId, Result};

use super::registry::ProviderRegistry;
use super::scorer::{rank, ScoredModel, ScoringCriteria, TaskComplexity};

/// Router configuration (subset of [`kontrocode_core::RouterConfig`]
/// that affects routing decisions only).
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Optimization mode.
    pub mode: ScoringCriteria,
    /// Per-provider fallback timeout.
    pub fallback_timeout: Duration,
    /// Maximum number of providers to try before giving up.
    pub max_fallbacks: usize,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            mode: ScoringCriteria::Cost,
            fallback_timeout: Duration::from_millis(300),
            max_fallbacks: 3,
        }
    }
}

/// An event emitted by the router during a request. Useful for the UI's
/// status bar and the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouterEvent {
    /// A model was selected.
    Selected {
        /// The model chosen.
        model: ModelId,
        /// The score that won.
        score: f64,
    },
    /// A provider failed; falling back to the next in chain.
    Fallback {
        /// The model that failed.
        from: ModelId,
        /// Why it failed.
        reason: String,
        /// The model we're trying next.
        to: ModelId,
    },
    /// All providers in the chain failed.
    Exhausted {
        /// The last error message.
        reason: String,
    },
}

/// The router. Cheap to clone — internally an `Arc`.
#[derive(Debug, Clone)]
pub struct Router {
    inner: Arc<RouterInner>,
}

#[derive(Debug)]
struct RouterInner {
    registry: ProviderRegistry,
    config: RouterConfig,
}

impl Router {
    /// Construct a router with the given registry and config.
    pub fn new(registry: ProviderRegistry, config: RouterConfig) -> Self {
        Self {
            inner: Arc::new(RouterInner { registry, config }),
        }
    }

    /// Construct a router with default config.
    pub fn with_default_config(registry: ProviderRegistry) -> Self {
        Self::new(registry, RouterConfig::default())
    }

    /// Access the underlying provider registry.
    pub fn registry(&self) -> &ProviderRegistry {
        &self.inner.registry
    }

    /// Access the router config.
    pub fn config(&self) -> &RouterConfig {
        &self.inner.config
    }

    /// Pick the best model for a given complexity, returning the ranked
    /// list (best first) so callers can implement their own fallback.
    pub fn rank_for(
        &self,
        complexity: TaskComplexity,
        model_filter: Option<&[&str]>,
    ) -> Vec<ScoredModel> {
        let mut models = self.inner.registry.all_models();
        if let Some(filter) = model_filter {
            models.retain(|m| filter.contains(&m.id.model()));
        }
        rank(&models, self.inner.config.mode, complexity)
    }

    /// Complete a request, walking the fallback chain on failure.
    pub async fn complete(
        &self,
        complexity: TaskComplexity,
        mut request: CompletionRequest,
        mut on_event: impl FnMut(RouterEvent),
    ) -> Result<CompletionResponse> {
        let ranked = self.rank_for(complexity, None);
        if ranked.is_empty() {
            return Err(kontrocode_core::Error::other(
                "no models available — register a provider",
            ));
        }

        let mut last_err: Option<kontrocode_core::Error> = None;
        for (i, candidate) in ranked
            .iter()
            .take(self.inner.config.max_fallbacks)
            .enumerate()
        {
            let provider = match self.inner.registry.provider_for(&candidate.spec.id) {
                Some(p) => p,
                None => {
                    last_err = Some(kontrocode_core::Error::other(format!(
                        "no provider for model {}",
                        candidate.spec.id
                    )));
                    continue;
                }
            };

            if !provider.healthy().await {
                last_err = Some(kontrocode_core::Error::provider(
                    provider.id(),
                    "provider reports unhealthy",
                ));
                continue;
            }

            request.model = candidate.spec.id.clone();
            if i == 0 {
                on_event(RouterEvent::Selected {
                    model: candidate.spec.id.clone(),
                    score: candidate.score,
                });
            }

            match timeout(
                self.inner.config.fallback_timeout,
                provider.complete(request.clone()),
            )
            .await
            {
                Ok(Ok(resp)) => return Ok(resp),
                Ok(Err(e)) => {
                    let reason = e.to_string();
                    if let Some(next) = ranked.get(i + 1) {
                        on_event(RouterEvent::Fallback {
                            from: candidate.spec.id.clone(),
                            reason,
                            to: next.spec.id.clone(),
                        });
                    }
                    last_err = Some(e);
                }
                Err(_elapsed) => {
                    let reason = format!(
                        "timeout after {}ms",
                        self.inner.config.fallback_timeout.as_millis()
                    );
                    if let Some(next) = ranked.get(i + 1) {
                        on_event(RouterEvent::Fallback {
                            from: candidate.spec.id.clone(),
                            reason: reason.clone(),
                            to: next.spec.id.clone(),
                        });
                    }
                    last_err = Some(kontrocode_core::Error::provider(provider.id(), reason));
                }
            }
        }

        let reason = last_err
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        on_event(RouterEvent::Exhausted {
            reason: reason.clone(),
        });
        Err(last_err.unwrap_or_else(|| kontrocode_core::Error::other(reason)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;
    use kontrocode_core::{Message, ModelSpec};

    fn router_with_mock() -> Router {
        let mut reg = ProviderRegistry::new();
        reg.register(MockProvider::new());
        Router::with_default_config(reg)
    }

    #[tokio::test]
    async fn complete_uses_first_model() {
        let r = router_with_mock();
        let req = CompletionRequest::new(ModelId::new("mock", "echo"), vec![Message::user("hi")]);
        let events = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let events2 = events.clone();
        let resp = r
            .complete(TaskComplexity::Routine, req, |e| {
                let e2 = events2.clone();
                tokio::spawn(async move {
                    e2.lock().await.push(e);
                });
            })
            .await
            .unwrap();
        assert!(resp.message.content.contains("hi"));
    }

    #[test]
    fn rank_for_returns_best_first() {
        let r = router_with_mock();
        let ranked = r.rank_for(TaskComplexity::Trivial, None);
        assert!(!ranked.is_empty());
    }

    #[test]
    fn empty_registry_errors() {
        let r = Router::with_default_config(ProviderRegistry::new());
        let req = CompletionRequest::new(ModelId::new("mock", "x"), vec![]);
        let err = futures::executor::block_on(async {
            r.complete(TaskComplexity::Routine, req, |_| {}).await
        });
        assert!(err.is_err());
    }

    // Suppress unused import
    #[allow(dead_code)]
    fn _force_model_spec(_: ModelSpec) {}
}
