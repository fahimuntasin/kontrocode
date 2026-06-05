//! The research runner — fans out to all applicable sources concurrently.

use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use kontrocode_core::{DecisionReport, ResearchCandidate, Result, Stack};
use tracing::{debug, warn};

use crate::cache::{CacheKey, InMemoryCache};
use crate::sources::ResearchSource;

/// Configuration for the research runner.
#[derive(Debug, Clone)]
pub struct ResearchRunnerConfig {
    /// Per-source timeout.
    pub source_timeout: Duration,
    /// Whether to include Stack Overflow.
    pub include_stack_overflow: bool,
    /// Whether to include GitHub signals.
    pub include_github: bool,
}

impl Default for ResearchRunnerConfig {
    fn default() -> Self {
        Self {
            source_timeout: Duration::from_secs(2),
            include_stack_overflow: true,
            include_github: true,
        }
    }
}

/// The runner. Cheap to clone.
#[derive(Clone)]
pub struct ResearchRunner {
    inner: Arc<ResearchRunnerInner>,
}

struct ResearchRunnerInner {
    sources: Vec<Arc<dyn ResearchSource>>,
    cache: InMemoryCache,
    config: ResearchRunnerConfig,
}

impl std::fmt::Debug for ResearchRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResearchRunner")
            .field("source_count", &self.inner.sources.len())
            .field("cache_size", &self.inner.cache.len())
            .finish()
    }
}

impl ResearchRunner {
    /// Construct a runner with the given sources and config.
    pub fn new(sources: Vec<Arc<dyn ResearchSource>>, config: ResearchRunnerConfig) -> Self {
        Self {
            inner: Arc::new(ResearchRunnerInner {
                sources,
                cache: InMemoryCache::default_ttl(),
                config,
            }),
        }
    }

    /// Construct a runner with no real sources (only [`NullSource`](crate::sources::NullSource)).
    pub fn empty() -> Self {
        Self::new(Vec::new(), ResearchRunnerConfig::default())
    }

    /// Access the underlying cache.
    pub fn cache(&self) -> &InMemoryCache {
        &self.inner.cache
    }

    /// Run all applicable sources concurrently and aggregate.
    ///
    /// `topic` is a free-form description of what we're researching
    /// (e.g. `"state management for Flutter"`). Each source filters
    /// and parses it as needed.
    pub async fn research(&self, stack: Stack, topic: &str) -> Result<DecisionReport> {
        let applicable: Vec<Arc<dyn ResearchSource>> = self
            .inner
            .sources
            .iter()
            .filter(|s| {
                let enabled = match s.id() {
                    "stackoverflow" => self.inner.config.include_stack_overflow,
                    "github" => self.inner.config.include_github,
                    _ => true,
                };
                enabled && s.supports(stack)
            })
            .cloned()
            .collect();

        debug!(
            stack = %stack.display_name(),
            topic = topic,
            source_count = applicable.len(),
            "research runner starting"
        );

        let cache = &self.inner.cache;
        let timeout = self.inner.config.source_timeout;

        let futures = applicable.into_iter().map(|source| {
            let topic = topic.to_string();
            let cache = cache.clone();
            async move {
                let key = InMemoryCache::key(source.id(), &stack.to_string(), &topic);
                if let Some(hit) = cache.get(&key) {
                    return (source.id(), Ok(hit));
                }
                let res = tokio::time::timeout(timeout, source.fetch(stack, &topic, &cache)).await;
                match res {
                    Ok(Ok(mut candidates)) => {
                        candidates.iter_mut().for_each(|c| {
                            if c.source.is_empty() {
                                c.source = source.id().to_string();
                            }
                        });
                        cache.put(key, candidates.clone());
                        (source.id(), Ok(candidates))
                    }
                    Ok(Err(e)) => (source.id(), Err(e)),
                    Err(_) => (
                        source.id(),
                        Err(kontrocode_core::Error::research(
                            source.id(),
                            "source timeout",
                        )),
                    ),
                }
            }
        });

        let results = join_all(futures).await;
        let mut all: Vec<ResearchCandidate> = Vec::new();
        for (id, res) in results {
            match res {
                Ok(mut cs) => all.append(&mut cs),
                Err(e) => warn!(source = id, error = %e, "research source failed"),
            }
        }

        Ok(aggregate(all))
    }
}

fn aggregate(candidates: Vec<ResearchCandidate>) -> DecisionReport {
    if candidates.is_empty() {
        return DecisionReport {
            confidence: 0.0,
            ..Default::default()
        };
    }
    let avg = candidates.iter().map(|c| c.score).sum::<f64>() / candidates.len() as f64;
    let max = candidates
        .iter()
        .max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();
    DecisionReport {
        confidence: avg,
        notes: vec![format!(
            "Aggregated {} candidates from all sources.",
            candidates.len()
        )],
        // Phase 1: bucket best-scoring into state_management as a placeholder.
        state_management: max,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::NullSource;

    #[tokio::test]
    async fn empty_runner_returns_empty_report() {
        let r = ResearchRunner::empty();
        let report = r.research(Stack::Flutter, "state mgmt").await.unwrap();
        assert_eq!(report.confidence, 0.0);
    }

    #[tokio::test]
    async fn null_source_runs_without_panicking() {
        let r = ResearchRunner::new(vec![Arc::new(NullSource)], ResearchRunnerConfig::default());
        let report = r.research(Stack::Flutter, "anything").await.unwrap();
        // NullSource returns no candidates, so report is empty.
        assert_eq!(report.confidence, 0.0);
    }
}
