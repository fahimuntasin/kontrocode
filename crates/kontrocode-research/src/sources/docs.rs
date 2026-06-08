use async_trait::async_trait;
use kontrocode_core::{ResearchCandidate, Result, Stack};
use reqwest::Client;
use serde::Deserialize;

use crate::cache::CacheKey;
use crate::sources::ResearchSource;

const FLUTTER_API: &str = "https://api.flutter.dev/flutter";
const DOCS_RS: &str = "https://docs.rs";
const NODEJS_DOCS: &str = "https://nodejs.org/api";

pub struct DocsScraper {
    client: Client,
}

impl DocsScraper {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    async fn scrape_flutter(&self, topic: &str) -> Result<Vec<ResearchCandidate>> {
        let class = topic.split_whitespace().next().unwrap_or(topic);
        let url = format!("{FLUTTER_API}/{class}/{class}-library.html");
        let resp = self.client.get(&url).send().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let html = resp.text().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;

        let mut candidates = Vec::new();

        for line in html.lines() {
            if line.contains("deprecated") || line.contains("Deprecated") {
                candidates.push(ResearchCandidate {
                    name: class.to_string(),
                    version: "latest".into(),
                    score: 0.1,
                    reason: format!("DEPRECATED at flutter.dev: {}", line.trim()),
                    source: "flutter.dev".into(),
                    url: Some(url.clone()),
                });
            }
            if line.contains("class ") && line.contains(class) {
                candidates.push(ResearchCandidate {
                    name: class.to_string(),
                    version: "latest".into(),
                    score: 0.95,
                    reason: format!("Official Flutter class documented at flutter.dev"),
                    source: "flutter.dev".into(),
                    url: Some(url.clone()),
                });
                break;
            }
        }
        Ok(candidates)
    }

    async fn scrape_docs_rs(&self, topic: &str) -> Result<Vec<ResearchCandidate>> {
        let crate_name = topic.split_whitespace().next().unwrap_or(topic);
        let url = format!("{DOCS_RS}/{crate_name}/latest/{crate_name}/index.html");
        let resp = self.client.get(&url).send().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;

        if resp.status().is_success() {
            Ok(vec![ResearchCandidate {
                name: crate_name.to_string(),
                version: "latest".into(),
                score: 0.9,
                reason: format!("Official Rust docs at docs.rs/{crate_name}"),
                source: "docs.rs".into(),
                url: Some(url),
            }])
        } else {
            Ok(vec![ResearchCandidate {
                name: crate_name.to_string(),
                version: "unknown".into(),
                score: 0.3,
                reason: format!("No docs.rs entry for {crate_name}"),
                source: "docs.rs".into(),
                url: None,
            }])
        }
    }

    async fn scrape_nodejs(&self, topic: &str) -> Result<Vec<ResearchCandidate>> {
        let module = topic.split_whitespace().next().unwrap_or(topic);
        let url = format!("{NODEJS_DOCS}/{module}.html");
        let resp = self.client.get(&url).send().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;

        if resp.status().is_success() {
            let html = resp.text().await.unwrap_or_default();
            let stability = if html.contains("Stability: 1") || html.contains("Stable") {
                "stable"
            } else if html.contains("Stability: 0") || html.contains("Deprecated") {
                "deprecated"
            } else {
                "unknown"
            };

            Ok(vec![ResearchCandidate {
                name: module.to_string(),
                version: "latest".into(),
                score: if stability == "deprecated" { 0.1 } else { 0.85 },
                reason: format!("nodejs.org docs: {module} ({stability})"),
                source: "nodejs.org".into(),
                url: Some(url),
            }])
        } else {
            Ok(Vec::new())
        }
    }
}

#[async_trait]
impl ResearchSource for DocsScraper {
    fn id(&self) -> &'static str { "docs" }

    async fn fetch(&self, stack: Stack, topic: &str, _cache: &dyn CacheKey) -> Result<Vec<ResearchCandidate>> {
        match stack {
            Stack::Flutter => self.scrape_flutter(topic).await,
            Stack::Rust => self.scrape_docs_rs(topic).await,
            Stack::Node => self.scrape_nodejs(topic).await,
            _ => Ok(Vec::new()),
        }
    }
}
