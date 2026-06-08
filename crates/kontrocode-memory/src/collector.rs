use chrono::Utc;
use kontrocode_core::{Fact, FactSource};
use serde_json::json;
use tracing::debug;
use uuid::Uuid;

use super::signal::Signal;

const SIGNAL_STREAM: &str = "kontrocode:signals";
const MAX_BATCH: usize = 100;

pub struct SignalCollector {
    redis_url: String,
    buffer: Vec<Signal>,
}

impl SignalCollector {
    pub fn new(redis_url: String) -> Self {
        Self {
            redis_url,
            buffer: Vec::with_capacity(MAX_BATCH),
        }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("KONTROCODE_REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        Self::new(url)
    }

    pub fn record(&mut self, signal: Signal) {
        self.buffer.push(signal);
        if self.buffer.len() >= MAX_BATCH {
            let signals = std::mem::take(&mut self.buffer);
            self.flush_to_redis(signals);
        }
    }

    fn flush_to_redis(&self, signals: Vec<Signal>) {
        let redis_url = self.redis_url.clone();
        tokio::spawn(async move {
            let client = match redis::Client::open(redis_url.clone()) {
                Ok(c) => c,
                Err(_) => return,
            };
            let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
                return;
            };
            for signal in &signals {
                let json = serde_json::to_string(signal).unwrap_or_default();
                let _: Result<(), _> = redis::AsyncCommands::xadd(
                    &mut conn,
                    SIGNAL_STREAM,
                    "*",
                    &[("signal", json.as_str())],
                )
                .await;
            }
            debug!("flushed {} signals to redis", signals.len());
        });
    }

    pub fn flush(&mut self) {
        let signals = std::mem::take(&mut self.buffer);
        if !signals.is_empty() {
            self.flush_to_redis(signals);
        }
    }
}

impl Drop for SignalCollector {
    fn drop(&mut self) {
        self.flush();
    }
}

pub fn extract_fact_from_signal(signal: &Signal) -> Option<Fact> {
    match signal.kind {
        super::signal::SignalKind::CodeBlockCopied => signal.topic.as_ref().map(|topic| Fact {
            id: Uuid::new_v4().to_string(),
            text: format!("copies code about {topic}"),
            confidence: 0.7,
            created_at: Utc::now(),
            source: FactSource::Implicit,
            embedding: None,
        }),
        super::signal::SignalKind::LibraryReplaced => {
            signal.meta.get("replacement").and_then(|r| r.as_str()).map(|lib| Fact {
                id: Uuid::new_v4().to_string(),
                text: format!("prefers {lib}"),
                confidence: 0.85,
                created_at: Utc::now(),
                source: FactSource::Implicit,
                embedding: None,
            })
        }
        super::signal::SignalKind::ExplicitStackMention => signal.topic.as_ref().map(|topic| Fact {
            id: Uuid::new_v4().to_string(),
            text: format!("uses {topic}"),
            confidence: 0.95,
            created_at: Utc::now(),
            source: FactSource::Explicit,
            embedding: None,
        }),
        super::signal::SignalKind::RecurringError => {
            signal.meta.get("error").and_then(|e| e.as_str()).map(|err| Fact {
                id: Uuid::new_v4().to_string(),
                text: format!("frequently encounters: {err}"),
                confidence: 0.6,
                created_at: Utc::now(),
                source: FactSource::Implicit,
                embedding: None,
            })
        }
        _ => None,
    }
}
