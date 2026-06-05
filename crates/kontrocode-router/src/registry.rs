//! Provider registry — collection of registered [`Provider`]s.

use std::collections::HashMap;
use std::sync::Arc;

use kontrocode_core::{ModelId, ModelSpec};

use super::Provider;

/// A thread-safe registry of LLM providers keyed by their [`Provider::id`].
#[derive(Default, Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRegistry")
            .field("ids", &self.ids())
            .finish()
    }
}

impl ProviderRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider. Replaces any existing provider with the same id.
    pub fn register<P: Provider + 'static>(&mut self, provider: P) -> &mut Self {
        self.providers
            .insert(provider.id().to_string(), Arc::new(provider));
        self
    }

    /// Get a provider by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(id).cloned()
    }

    /// All registered provider ids.
    pub fn ids(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.providers.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }

    /// All available model specs across all providers.
    pub fn all_models(&self) -> Vec<ModelSpec> {
        self.providers
            .values()
            .flat_map(|p| p.models().to_vec())
            .collect()
    }

    /// Look up the provider that serves a given model id.
    pub fn provider_for(&self, model: &ModelId) -> Option<Arc<dyn Provider>> {
        self.providers.get(model.provider()).cloned()
    }

    /// Number of registered providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Shutdown all providers.
    pub async fn shutdown_all(&self) {
        for p in self.providers.values() {
            p.shutdown().await;
        }
    }

    /// Construct from a list of providers.
    pub fn from_providers(providers: Vec<Arc<dyn Provider>>) -> Self {
        let mut r = Self::new();
        for p in providers {
            r.providers.insert(p.id().to_string(), p);
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;

    #[test]
    fn register_and_lookup() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::new());
        assert_eq!(r.ids(), vec!["mock"]);
        assert!(r.get("mock").is_some());
        assert!(r.get("missing").is_none());
    }

    #[test]
    fn all_models_aggregates() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::new());
        assert_eq!(r.all_models().len(), 1);
    }

    #[tokio::test]
    async fn shutdown_all_is_noop_for_mock() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::new());
        r.shutdown_all().await;
    }
}
