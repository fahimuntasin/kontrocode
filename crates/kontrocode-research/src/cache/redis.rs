use async_trait::async_trait;
use kontrocode_core::ResearchCandidate;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use tracing::{debug, info};

use crate::cache::CacheKey;
use crate::sources::ResearchSource;

const CACHE_PREFIX: &str = "kontrocode:research";
const DEFAULT_TTL: u64 = 86_400;

pub struct RedisResearchCache {
    conn: MultiplexedConnection,
    ttl_seconds: u64,
    key_prefix: String,
}

impl RedisResearchCache {
    pub async fn connect(redis_url: &str, ttl_seconds: u64) -> Result<Self, kontrocode_core::Error> {
        let client = Client::open(redis_url)
            .map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        info!("redis research cache connected: {redis_url} (TTL={ttl_seconds}s)");
        Ok(Self {
            conn,
            ttl_seconds,
            key_prefix: CACHE_PREFIX.to_string(),
        })
    }

    pub async fn default_connect() -> Result<Self, kontrocode_core::Error> {
        let url = std::env::var("KONTROCODE_REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        Self::connect(&url, DEFAULT_TTL).await
    }

    pub fn full_key(&self, source: &str, stack: &str, topic: &str) -> String {
        format!("{}:{}:{}:{}", self.key_prefix, source, stack, topic)
    }

    pub async fn get_cached(
        &self,
        source: &str,
        stack: &str,
        topic: &str,
    ) -> Option<Vec<ResearchCandidate>> {
        let key = self.full_key(source, stack, topic);
        let mut conn = self.conn.clone();
        let json: Option<String> = conn.get(&key).await.ok()?;
        json.and_then(|s| serde_json::from_str(&s).ok())
    }

    pub async fn set_cached(
        &self,
        source: &str,
        stack: &str,
        topic: &str,
        candidates: &[ResearchCandidate],
    ) -> Result<(), kontrocode_core::Error> {
        let key = self.full_key(source, stack, topic);
        let json = serde_json::to_string(candidates)
            .map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let mut conn = self.conn.clone();
        let _: () = redis::pipe()
            .set(&key, &json)
            .expire(&key, self.ttl_seconds as i64)
            .query_async(&mut conn)
            .await
            .map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        debug!("research cache set: {key}");
        Ok(())
    }
}

pub struct CachedSource<S: ResearchSource> {
    inner: S,
    cache: RedisResearchCache,
    cache_ttl: u64,
}

impl<S: ResearchSource> CachedSource<S> {
    pub fn new(source: S, cache: RedisResearchCache) -> Self {
        Self {
            inner: source,
            cache,
            cache_ttl: DEFAULT_TTL,
        }
    }
}

#[async_trait]
impl<S: ResearchSource> ResearchSource for CachedSource<S> {
    fn id(&self) -> &'static str {
        self.inner.id()
    }

    fn supports(&self, stack: kontrocode_core::Stack) -> bool {
        self.inner.supports(stack)
    }

    async fn fetch(
        &self,
        stack: kontrocode_core::Stack,
        topic: &str,
        _cache: &dyn CacheKey,
    ) -> kontrocode_core::Result<Vec<ResearchCandidate>> {
        let stack_str = format!("{:?}", stack);
        if let Some(cached) = self
            .cache
            .get_cached(self.id(), &stack_str, topic)
            .await
        {
            debug!(source = self.id(), "research cache HIT");
            return Ok(cached);
        }
        debug!(source = self.id(), "research cache MISS");
        let results = self.inner.fetch(stack, topic, _cache).await?;
        let _ = self
            .cache
            .set_cached(self.id(), &stack_str, topic, &results)
            .await;
        Ok(results)
    }
}
