use std::fmt;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ResearchEvent {
    pub source: String,
    pub status: ResearchStatus,
    pub message: String,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResearchStatus {
    Started,
    Fetching,
    CacheHit,
    CacheMiss,
    Done(usize),
    Error(String),
}

impl fmt::Display for ResearchStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResearchStatus::Started => write!(f, "▶"),
            ResearchStatus::Fetching => write!(f, "↓"),
            ResearchStatus::CacheHit => write!(f, "📦"),
            ResearchStatus::CacheMiss => write!(f, "🌐"),
            ResearchStatus::Done(n) => write!(f, "✓({n})"),
            ResearchStatus::Error(_) => write!(f, "✗"),
        }
    }
}

pub struct ResearchFeed {
    events: Vec<ResearchEvent>,
    started_at: Instant,
}

impl ResearchFeed {
    pub fn new() -> Self {
        Self { events: Vec::new(), started_at: Instant::now() }
    }

    pub fn push(&mut self, source: &str, status: ResearchStatus, message: &str) {
        self.events.push(ResearchEvent {
            source: source.to_string(),
            status,
            message: message.to_string(),
            timestamp: Instant::now(),
        });
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("\n┌─ Research Feed ──────────────────────────────┐\n");

        if self.events.is_empty() {
            output.push_str("│  (no research yet)                              │\n");
        } else {
            for event in self.events.iter().rev().take(10) {
                let elapsed = event.timestamp.duration_since(self.started_at);
                let ms = elapsed.as_millis();
                let msg = if event.message.len() > 40 {
                    format!("{}...", &event.message[..37])
                } else {
                    event.message.clone()
                };
                output.push_str(&format!(
                    "│ {} {:>5}ms {:<12} {} │\n",
                    event.status, ms, event.source, msg
                ));
            }
        }

        output.push_str("└────────────────────────────────────────────────┘\n");

        let total = self.events.len();
        let done = self.events.iter().filter(|e| matches!(e.status, ResearchStatus::Done(_))).count();
        let errors = self.events.iter().filter(|e| matches!(e.status, ResearchStatus::Error(_))).count();
        output.push_str(&format!("  {total} sources | {done} done | {errors} errors\n"));

        output
    }

    pub fn render_compact(&self) -> String {
        let mut parts = Vec::new();
        for event in &self.events {
            match &event.status {
                ResearchStatus::Done(n) => parts.push(format!("{}({n})", event.source)),
                ResearchStatus::Error(e) => parts.push(format!("{}:✗", event.source)),
                _ => parts.push(format!("{}:{}", event.source, event.status)),
            }
        }
        format!("Research: {}", parts.join(" → "))
    }

    pub fn accordion_json(&self) -> serde_json::Value {
        let items: Vec<serde_json::Value> = self.events.iter().map(|e| {
            serde_json::json!({
                "source": e.source,
                "type": match &e.status {
                    ResearchStatus::Done(_) => "complete",
                    ResearchStatus::Error(_) => "error",
                    ResearchStatus::CacheHit => "cached",
                    _ => "fetching",
                },
                "result": e.message,
            })
        }).collect();

        serde_json::json!({
            "total_sources": self.events.len(),
            "items": items,
        })
    }
}
