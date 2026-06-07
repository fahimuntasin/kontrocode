//! Research source trait + null/empty implementation for Phase 1.

use async_trait::async_trait;
use kontrocode_core::{Result, Stack};

use crate::cache::CacheKey;

/// A single research source (pub.dev, npm, official docs, GitHub, …).
///
/// Implementations are stateless beyond the cache. The runner handles
/// concurrency and error isolation.
#[async_trait]
pub trait ResearchSource: Send + Sync {
    /// Stable, lowercase identifier.
    fn id(&self) -> &'static str;

    /// Whether this source supports a given stack. Default: yes for
    /// `Unknown`, no otherwise. Override for stack-specific sources.
    fn supports(&self, stack: Stack) -> bool {
        matches!(stack, Stack::Unknown)
    }

    /// Fetch a research result. Implementations should:
    /// 1. Build a [`CacheKey`].
    /// 2. Check the cache; return early on hit.
    /// 3. Otherwise fetch, parse, score, and return candidates.
    async fn fetch(
        &self,
        stack: Stack,
        topic: &str,
        cache: &dyn CacheKey,
    ) -> Result<Vec<kontrocode_core::ResearchCandidate>>;
}

/// A null/empty source that always returns no results. Used in Phase 1
/// before real fetchers land, and useful in tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSource;

#[async_trait]
impl ResearchSource for NullSource {
    fn id(&self) -> &'static str {
        "null"
    }

    fn supports(&self, _: Stack) -> bool {
        true
    }

    async fn fetch(
        &self,
        _: Stack,
        _: &str,
        _: &dyn CacheKey,
    ) -> Result<Vec<kontrocode_core::ResearchCandidate>> {
        Ok(Vec::new())
    }
}
