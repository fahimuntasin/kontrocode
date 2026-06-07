use async_trait::async_trait;
use kontrocode_core::{ResearchCandidate, Result, Stack};
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

use crate::cache::CacheKey;
use crate::sources::ResearchSource;

pub struct PubDevSource {
    client: Client,
}

impl PubDevSource {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

#[async_trait]
impl ResearchSource for PubDevSource {
    fn id(&self) -> &'static str { "pub.dev" }
    fn supports(&self, stack: Stack) -> bool { matches!(stack, Stack::Flutter) }

    async fn fetch(&self, _s: Stack, topic: &str, _cache: &dyn CacheKey) -> Result<Vec<ResearchCandidate>> {
        let name = topic.split_whitespace().next().unwrap_or(topic);
        let url = format!("https://pub.dev/api/packages/{name}");
        let resp = self.client.get(&url).send().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let body: PdResp = resp.json().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let latest = body.latest_version();
        let score = (body.total_versions().min(100) as f64 / 100.0).min(1.0);
        info!(package = %body.name, version = %latest, score = %score, "pub.dev: done");
        Ok(vec![ResearchCandidate {
            name: body.name,
            version: latest,
            score,
            reason: body.latest.description.unwrap_or_default(),
            source: "pub.dev".into(),
            url: Some(format!("https://pub.dev/packages/{}", body.latest.name)),
        }])
    }
}

#[derive(Debug, Deserialize)]
struct PdResp {
    name: String,
    latest: PdLatest,
    versions: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PdLatest {
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
}

impl PdResp {
    fn latest_version(&self) -> String { self.latest.version.clone() }
    fn total_versions(&self) -> usize { self.versions.len() }
}

pub struct NpmSource {
    client: Client,
}

impl NpmSource {
    pub fn new() -> Self { Self { client: Client::new() } }
}

#[async_trait]
impl ResearchSource for NpmSource {
    fn id(&self) -> &'static str { "npm" }
    fn supports(&self, stack: Stack) -> bool {
        matches!(stack, Stack::Node | Stack::React | Stack::Vue | Stack::Svelte)
    }

    async fn fetch(&self, _s: Stack, topic: &str, _cache: &dyn CacheKey) -> Result<Vec<ResearchCandidate>> {
        let name = topic.split_whitespace().next().unwrap_or(topic);
        let url = format!("https://registry.npmjs.org/{name}");
        let resp = self.client.get(&url).send().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let body: NpmResp = resp.json().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let latest = body.latest_version();
        let deprecated = body.deprecated().unwrap_or_default();
        let score = if deprecated { 0.1 } else { 0.9 };
        info!(package = %body.name, version = %latest, score = %score, "npm: done");
        Ok(vec![ResearchCandidate {
            name: body.name,
            version: latest,
            score,
            reason: body.description.unwrap_or_default(),
            source: "npm".into(),
            url: Some(format!("https://www.npmjs.com/package/{name}")),
        }])
    }
}

#[derive(Debug, Deserialize)]
struct NpmResp {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    deprecated: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<serde_json::Value>,
}

impl NpmResp {
    fn latest_version(&self) -> String {
        self.dist_tags.as_ref()
            .and_then(|t| t.get("latest"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    }
    fn deprecated(&self) -> Option<bool> { self.deprecated.as_ref().map(|_| true) }
}

pub struct CratesIoSource {
    client: Client,
}

impl CratesIoSource {
    pub fn new() -> Self { Self { client: Client::new() } }
}

#[async_trait]
impl ResearchSource for CratesIoSource {
    fn id(&self) -> &'static str { "crates.io" }
    fn supports(&self, stack: Stack) -> bool { matches!(stack, Stack::Rust) }

    async fn fetch(&self, _s: Stack, topic: &str, _cache: &dyn CacheKey) -> Result<Vec<ResearchCandidate>> {
        let name = topic.split_whitespace().next().unwrap_or(topic);
        let url = format!("https://crates.io/api/v1/crates/{name}");
        let resp = self.client.get(&url).send().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let body: CrResp = resp.json().await.map_err(|e| kontrocode_core::Error::other(e.to_string()))?;
        let score = (body.cr.recent_downloads.unwrap_or(0) as f64 / 1_000_000.0).min(1.0);
        info!(crate_name = %body.cr.name, score = %score, "crates.io: done");
        Ok(vec![ResearchCandidate {
            name: body.cr.name,
            version: body.cr.max_stable_version.unwrap_or_else(|| "unknown".into()),
            score,
            reason: body.cr.description.unwrap_or_default(),
            source: "crates.io".into(),
            url: Some(format!("https://crates.io/crates/{}", body.cr.id)),
        }])
    }
}

#[derive(Debug, Deserialize)]
struct CrResp {
    #[serde(rename = "crate")]
    cr: CrData,
}

#[derive(Debug, Deserialize)]
struct CrData {
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    max_stable_version: Option<String>,
    #[serde(default)]
    recent_downloads: Option<u64>,
}
