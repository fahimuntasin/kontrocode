//! File-backed [`MemoryStore`] implementation. Default for Phase 1.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::Utc;
use kontrocode_core::{Fact, Profile, Result};
use parking_lot::Mutex;
use tracing::{debug, info, warn};

use super::store::MemoryStore;

/// Persists the profile as a single JSON file. Atomic writes via
/// tempfile + rename.
#[derive(Debug)]
pub struct FileMemoryStore {
    path: PathBuf,
    cache: Mutex<Option<Profile>>,
}

impl FileMemoryStore {
    /// Construct a file store at the given path. The file is created on
    /// first save if it doesn't exist.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            cache: Mutex::new(None),
        }
    }

    /// Construct a file store at the default config path.
    pub fn default_location() -> Self {
        Self::new(super::store::default_profile_path())
    }

    fn ensure_parent(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

#[async_trait]
impl MemoryStore for FileMemoryStore {
    async fn load(&self) -> Result<Profile> {
        if let Some(p) = self.cache.lock().clone() {
            return Ok(p);
        }
        if !self.path.exists() {
            debug!(path = %self.path.display(), "no profile yet, returning default");
            return Ok(Profile::default());
        }
        let raw = std::fs::read_to_string(&self.path)?;
        let profile: Profile = match serde_json::from_str(&raw) {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, path = %self.path.display(), "profile is corrupt — starting fresh");
                Profile::default()
            }
        };
        *self.cache.lock() = Some(profile.clone());
        Ok(profile)
    }

    async fn save(&self, profile: &Profile) -> Result<()> {
        self.ensure_parent()?;
        let mut profile = profile.clone();
        profile.last_updated = Utc::now();
        let json = serde_json::to_string_pretty(&profile)?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &self.path)?;
        *self.cache.lock() = Some(profile);
        info!(path = %self.path.display(), "profile saved");
        Ok(())
    }

    async fn update_fact(&self, id: &str, text: String) -> Result<()> {
        let mut profile = self.load().await?;
        if let Some(f) = profile.facts.iter_mut().find(|f| f.id == id) {
            f.text = text;
        } else {
            return Err(kontrocode_core::Error::memory(format!(
                "fact {id} not found"
            )));
        }
        self.save(&profile).await
    }

    async fn delete_fact(&self, id: &str) -> Result<()> {
        let mut profile = self.load().await?;
        let before = profile.facts.len();
        profile.facts.retain(|f| f.id != id);
        if profile.facts.len() == before {
            return Err(kontrocode_core::Error::memory(format!(
                "fact {id} not found"
            )));
        }
        self.save(&profile).await
    }

    async fn add_fact(&self, fact: Fact) -> Result<()> {
        let mut profile = self.load().await?;
        profile.facts.push(fact);
        self.save(&profile).await
    }

    fn location(&self) -> String {
        Path::new(&self.path).display().to_string()
    }

    async fn search_facts(&self, query: &str, top_k: usize) -> Result<Vec<Fact>> {
        let profile = self.load().await?;
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let has_embeddings = profile.facts.iter().any(|f| f.embedding.is_some());

        let mut scored: Vec<(f64, Fact)> = profile
            .facts
            .into_iter()
            .map(|f| {
                let score = if has_embeddings {
                    let text_lower = f.text.to_lowercase();
                    let exact = if text_lower.contains(&query_lower) { 2.0 } else { 0.0 };
                    let word_hits = query_words
                        .iter()
                        .filter(|w| text_lower.contains(**w))
                        .count() as f64;
                    (exact + word_hits * 0.5) * f.confidence as f64
                } else {
                    let text_lower = f.text.to_lowercase();
                    let exact = if text_lower.contains(&query_lower) { 3.0 } else { 0.0 };
                    let word_hits = query_words
                        .iter()
                        .filter(|w| text_lower.contains(**w))
                        .count() as f64;
                    (exact + word_hits * 0.7) * f.confidence as f64
                };
                (score, f)
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).map(|(_, f)| f).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontrocode_core::FactSource;
    use tempfile::TempDir;

    fn temp_store() -> (TempDir, FileMemoryStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        (dir, FileMemoryStore::new(path))
    }

    #[tokio::test]
    async fn load_returns_default_when_no_file() {
        let (_dir, store) = temp_store();
        let p = store.load().await.unwrap();
        assert!(p.facts.is_empty());
    }

    #[tokio::test]
    async fn save_then_load_round_trips() {
        let (_dir, store) = temp_store();
        let p = Profile {
            summary: "Test user".into(),
            ..Default::default()
        };
        store.save(&p).await.unwrap();
        let loaded = store.load().await.unwrap();
        assert_eq!(loaded.summary, "Test user");
    }

    #[tokio::test]
    async fn add_fact_persists() {
        let (_dir, store) = temp_store();
        let fact = Fact {
            id: "f1".into(),
            text: "prefers Riverpod".into(),
            confidence: 0.9,
            created_at: Utc::now(),
            source: FactSource::Implicit,
            embedding: None,
        };
        store.add_fact(fact).await.unwrap();
        let loaded = store.load().await.unwrap();
        assert_eq!(loaded.facts.len(), 1);
        assert_eq!(loaded.facts[0].text, "prefers Riverpod");
    }

    #[tokio::test]
    async fn update_fact_modifies_in_place() {
        let (_dir, store) = temp_store();
        store
            .add_fact(Fact {
                id: "f1".into(),
                text: "old".into(),
                confidence: 0.5,
                created_at: Utc::now(),
                source: FactSource::Implicit,
            embedding: None,
            })
            .await
            .unwrap();
        store.update_fact("f1", "new".into()).await.unwrap();
        let loaded = store.load().await.unwrap();
        assert_eq!(loaded.facts[0].text, "new");
    }

    #[tokio::test]
    async fn delete_fact_removes() {
        let (_dir, store) = temp_store();
        store
            .add_fact(Fact {
                id: "f1".into(),
                text: "x".into(),
                confidence: 0.5,
                created_at: Utc::now(),
                source: FactSource::Implicit,
            embedding: None,
            })
            .await
            .unwrap();
        store.delete_fact("f1").await.unwrap();
        let loaded = store.load().await.unwrap();
        assert!(loaded.facts.is_empty());
    }

    #[tokio::test]
    async fn missing_fact_update_errors() {
        let (_dir, store) = temp_store();
        let err = store.update_fact("nope", "x".into()).await;
        assert!(err.is_err());
    }
}
