//! The [`MemoryStore`] trait — every backend implements this.

use std::path::PathBuf;

use async_trait::async_trait;
use kontrocode_core::{Fact, Profile, Result};

/// Backend-independent memory operations.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Load the full profile. Returns an empty default if none exists yet.
    async fn load(&self) -> Result<Profile>;

    /// Persist the profile. Atomic write.
    async fn save(&self, profile: &Profile) -> Result<()>;

    /// Update an existing fact in-place, or no-op if not found.
    async fn update_fact(&self, id: &str, text: String) -> Result<()>;

    /// Delete a fact by id. No-op if not found.
    async fn delete_fact(&self, id: &str) -> Result<()>;

    /// Add a new fact. Caller is responsible for dedup / confidence.
    async fn add_fact(&self, fact: Fact) -> Result<()>;

    /// Search facts by text similarity. Returns top-K matches sorted by relevance.
    /// Phase 4 default: simple substring + keyword overlap scoring.
    /// Phase 4 Redis: cosine similarity via RediSearch.
    async fn search_facts(&self, query: &str, top_k: usize) -> Result<Vec<Fact>>;

    /// Search facts using an embedding for cosine similarity.
    /// Default: falls back to empty query search.
    async fn semantic_search(
        &self,
        _query_embedding: &kontrocode_core::embedding::Embedding,
        top_k: usize,
    ) -> Result<Vec<Fact>> {
        self.search_facts("", top_k).await
    }

    /// The path or address of the underlying storage. For diagnostics.
    fn location(&self) -> String;
}

/// Helper for resolving a sensible default profile path.
pub fn default_profile_path() -> PathBuf {
    if let Ok(p) = std::env::var("KONTROCODE_PROFILE_PATH") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("kontrocode").join("profile.json")
}
