//! Cache key trait + in-memory implementation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Minimal interface a cache must provide to the research sources.
pub trait CacheKey: Send + Sync {
    /// Build a deterministic key for a (source, stack, topic) tuple.
    fn key(&self, source: &str, stack: &str, topic: &str) -> String {
        format!("research:{source}:{stack}:{}", normalize(topic))
    }
}

/// Cached entry: timestamp + candidates.
pub type CachedEntry = (Instant, Vec<kontrocode_core::ResearchCandidate>);

/// In-memory cache with TTL. Phase 1 default. Phase 4 swaps for Redis.
#[derive(Debug, Clone)]
pub struct InMemoryCache {
    ttl: Duration,
    inner: Arc<Mutex<HashMap<String, CachedEntry>>>,
}

impl InMemoryCache {
    /// Construct a new in-memory cache with the given TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Construct with the default TTL of 24 hours.
    pub fn default_ttl() -> Self {
        Self::new(Duration::from_secs(24 * 3600))
    }

    /// Get a cached value if present and not expired.
    pub fn get(&self, key: &str) -> Option<Vec<kontrocode_core::ResearchCandidate>> {
        let inner = self.inner.lock();
        inner.get(key).and_then(|(at, v)| {
            if at.elapsed() < self.ttl {
                Some(v.clone())
            } else {
                None
            }
        })
    }

    /// Insert a value with the current timestamp.
    pub fn put(&self, key: String, value: Vec<kontrocode_core::ResearchCandidate>) {
        self.inner.lock().insert(key, (Instant::now(), value));
    }

    /// Number of live entries.
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

impl CacheKey for InMemoryCache {}

fn normalize(topic: &str) -> String {
    topic
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontrocode_core::ResearchCandidate;

    #[test]
    fn put_then_get() {
        let c = InMemoryCache::new(Duration::from_secs(60));
        c.put(
            "k".into(),
            vec![ResearchCandidate {
                name: "x".into(),
                version: "1.0".into(),
                score: 0.9,
                reason: "test".into(),
                source: "test".into(),
                url: None,
            }],
        );
        let v = c.get("k").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].name, "x");
    }

    #[test]
    fn expired_entries_are_not_returned() {
        let c = InMemoryCache::new(Duration::from_millis(10));
        c.put("k".into(), vec![]);
        std::thread::sleep(Duration::from_millis(20));
        assert!(c.get("k").is_none());
    }

    #[test]
    fn key_is_deterministic_and_normalized() {
        let k1 = InMemoryCache::default_ttl().key("src", "stack", "Hello World!");
        let k2 = InMemoryCache::default_ttl().key("src", "stack", "hello world!");
        assert_eq!(k1, k2);
        assert!(k1.starts_with("research:src:stack:"));
    }
}
