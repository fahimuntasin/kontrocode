use async_trait::async_trait;
use kontrocode_core::{Fact, Profile, Result};
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use tracing::{debug, info, warn};

use crate::store::MemoryStore;

const PROFILE_KEY: &str = "kontrocode:profile:default";
const FACTS_HASH: &str = "kontrocode:facts";

pub struct RedisMemoryStore {
    conn: MultiplexedConnection,
    redis_url: String,
}

impl RedisMemoryStore {
    pub async fn connect(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url).map_err(|e| {
            kontrocode_core::Error::memory(format!("redis open: {e}"))
        })?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| kontrocode_core::Error::memory(format!("redis connect: {e}")))?;
        info!("redis connected: {redis_url}");
        Ok(Self {
            conn,
            redis_url: redis_url.to_string(),
        })
    }

    pub fn from_env() -> Option<String> {
        std::env::var("KONTROCODE_REDIS_URL")
            .ok()
            .or_else(|| Some("redis://127.0.0.1:6379".to_string()))
    }
}

#[async_trait]
impl MemoryStore for RedisMemoryStore {
    async fn load(&self) -> Result<Profile> {
        let mut conn = self.conn.clone();
        let json: Option<String> = conn.get(PROFILE_KEY).await.map_err(|e| {
            kontrocode_core::Error::memory(format!("redis get: {e}"))
        })?;
        match json {
            Some(s) => {
                let profile: Profile = serde_json::from_str(&s)
                    .map_err(|e| kontrocode_core::Error::memory(format!("profile parse: {e}")))?;
                debug!("profile loaded from redis");
                Ok(profile)
            }
            None => {
                debug!("no profile in redis, returning default");
                Ok(Profile::default())
            }
        }
    }

    async fn save(&self, profile: &Profile) -> Result<()> {
        let mut conn = self.conn.clone();
        let mut profile = profile.clone();
        profile.last_updated = chrono::Utc::now();
        let json = serde_json::to_string(&profile)
            .map_err(|e| kontrocode_core::Error::memory(format!("profile serialize: {e}")))?;
        let _: () = conn.set(PROFILE_KEY, &json).await.map_err(|e| {
            kontrocode_core::Error::memory(format!("redis set: {e}"))
        })?;
        info!("profile saved to redis ({})", profile.facts.len());
        Ok(())
    }

    async fn update_fact(&self, id: &str, text: String) -> Result<()> {
        let mut profile = self.load().await?;
        if let Some(f) = profile.facts.iter_mut().find(|f| f.id == id) {
            f.text = text;
            self.save(&profile).await
        } else {
            Err(kontrocode_core::Error::memory(format!("fact {id} not found")))
        }
    }

    async fn delete_fact(&self, id: &str) -> Result<()> {
        let mut profile = self.load().await?;
        let before = profile.facts.len();
        profile.facts.retain(|f| f.id != id);
        if profile.facts.len() == before {
            return Err(kontrocode_core::Error::memory(format!("fact {id} not found")));
        }
        self.save(&profile).await
    }

    async fn add_fact(&self, fact: Fact) -> Result<()> {
        let mut profile = self.load().await?;
        profile.facts.push(fact);
        self.save(&profile).await
    }

    async fn search_facts(&self, query: &str, top_k: usize) -> Result<Vec<Fact>> {
        let profile = self.load().await?;
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut scored: Vec<(f64, Fact)> = profile
            .facts
            .iter()
            .map(|f| {
                let t = f.text.to_lowercase();
                let exact = if t.contains(&query_lower) { 2.0 } else { 0.0 };
                let hits = words.iter().filter(|w| t.contains(**w)).count() as f64;
                let score = (exact + hits * 0.5) * f.confidence as f64;
                (score, f.clone())
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).map(|(_, f)| f).collect())
    }

    fn location(&self) -> String {
        self.redis_url.clone()
    }
}
