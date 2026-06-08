//! Configuration loaded from `~/.config/kontrocode/config.toml`.
//!
//! All crates read configuration through the [`Config`] struct. The struct
//! is `Deserialize` only — mutations go through the config writer in
//! `kontrocode-agent` so all changes are audited.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Result;

/// Top-level configuration for the KontroCode installation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// User profile and memory settings.
    #[serde(default)]
    pub memory: MemoryConfig,

    /// Multi-provider router settings.
    #[serde(default)]
    pub router: RouterConfig,

    /// Research agent settings.
    #[serde(default)]
    pub research: ResearchConfig,

    /// Editor and UI settings.
    #[serde(default)]
    pub ui: UiConfig,
}

impl Config {
    /// Load configuration from the default path:
    /// `$XDG_CONFIG_HOME/kontrocode/config.toml` (or platform equivalent).
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path).map_err(crate::Error::Io)?;
        let cfg: Self = toml::from_str(&raw)
            .map_err(|e| crate::Error::config(format!("invalid config.toml: {e}")))?;
        Ok(cfg)
    }

    /// Persist this configuration to the default path.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(crate::Error::Io)?;
        }
        let raw = toml::to_string_pretty(self)
            .map_err(|e| crate::Error::other(format!("serialize config: {e}")))?;
        std::fs::write(&path, raw).map_err(crate::Error::Io)?;
        Ok(())
    }

    /// Returns the platform-appropriate configuration file path.
    pub fn config_path() -> PathBuf {
        let base = dirs_config();
        base.join("kontrocode").join("config.toml")
    }
}

/// Memory subsystem configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryConfig {
    /// Backend implementation: `file` (default) or `redis` (Phase 4).
    pub backend: MemoryBackend,

    /// Path to the profile JSON file (file backend only).
    pub profile_path: PathBuf,

    /// Daily decay rate applied to interest scores. Default 0.02 (i.e. × 0.98/day).
    pub decay_rate: f64,

    /// RAG: number of memories to inject per request.
    pub rag_top_k: usize,

    /// RAG: maximum tokens injected for memories.
    pub rag_max_tokens: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: MemoryBackend::File,
            profile_path: default_profile_path(),
            decay_rate: 0.02,
            rag_top_k: 5,
            rag_max_tokens: 300,
        }
    }
}

/// Memory store backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryBackend {
    /// JSON file on disk. Default for Phase 1.
    File,
    /// Redis with RedisJSON + RediSearch. Phase 4.
    Redis,
}

/// Multi-provider router configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterConfig {
    /// Default optimization mode.
    pub mode: RouterMode,

    /// Monthly budget cap in USD. `0.0` means unlimited.
    pub monthly_budget_usd: f64,

    /// Fallback timeout per provider in milliseconds.
    pub fallback_timeout_ms: u64,

    /// Maximum retries per provider before giving up.
    pub max_retries: u32,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            mode: RouterMode::Cost,
            monthly_budget_usd: 0.0,
            fallback_timeout_ms: 300,
            max_retries: 3,
        }
    }
}

/// Default router optimization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouterMode {
    /// Route to the cheapest sufficient model.
    Cost,
    /// Route to the lowest-latency sufficient model.
    Speed,
    /// Route to the highest-quality sufficient model.
    Quality,
}

/// Research agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchConfig {
    /// Cache TTL in hours.
    pub cache_ttl_hours: u64,

    /// Maximum parallel sources per query.
    pub max_parallel: usize,

    /// Whether to include Stack Overflow as a source.
    pub include_stack_overflow: bool,

    /// Whether to include GitHub signals.
    pub include_github: bool,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            cache_ttl_hours: 24,
            max_parallel: 5,
            include_stack_overflow: true,
            include_github: true,
        }
    }
}

/// UI configuration (persisted across sessions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    /// Whether the right panel (agent) is visible.
    pub agent_panel_visible: bool,

    /// Whether the left panel (file tree) is visible.
    pub file_tree_visible: bool,

    /// Whether the bottom panel (terminal) is visible.
    pub terminal_visible: bool,

    /// Font size in pixels.
    pub font_size_px: u8,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            agent_panel_visible: true,
            file_tree_visible: true,
            terminal_visible: true,
            font_size_px: 13,
        }
    }
}

fn dirs_config() -> PathBuf {
    std::env::var_os("KONTROCODE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .unwrap_or_else(|| {
            #[cfg(target_os = "macos")]
            {
                dirs_home().join("Library").join("Application Support")
            }
            #[cfg(target_os = "windows")]
            {
                std::env::var_os("APPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| dirs_home().join("AppData").join("Roaming"))
            }
            #[cfg(target_os = "linux")]
            {
                dirs_home().join(".config")
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
            {
                PathBuf::from(".")
            }
        })
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_profile_path() -> PathBuf {
    dirs_config().join("kontrocode").join("profile.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips() {
        let cfg = Config::default();
        let raw = toml::to_string(&cfg).unwrap();
        let back: Config = toml::from_str(&raw).unwrap();
        assert_eq!(back.memory.decay_rate, cfg.memory.decay_rate);
        assert_eq!(back.router.mode, cfg.router.mode);
    }

    #[test]
    fn config_path_is_inside_kontrocode_dir() {
        let p = Config::config_path();
        assert!(p.ends_with("kontrocode/config.toml"));
    }
}
pub mod mcp;
