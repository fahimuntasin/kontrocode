//! Research agent contract.
//!
//! The research agent queries external sources in parallel before the
//! model generates code. This module defines the shared types every
//! research source produces and the decision engine consumes.

use serde::{Deserialize, Serialize};

use crate::Stack;

/// A scored candidate library / API / pattern from the research agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchCandidate {
    /// Canonical name (e.g. `"google_sign_in"`, `"riverpod"`).
    pub name: String,
    /// Version string. May be `"unknown"` if not pinned.
    pub version: String,
    /// Final composite score in `0.0..=1.0`.
    pub score: f64,
    /// Why this score. Always include a short reason.
    pub reason: String,
    /// Where this candidate came from.
    pub source: String,
    /// Optional registry URL for the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A consolidated report from the research agent's decision engine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionReport {
    /// Best library for the state-management slot (or `None`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_management: Option<ResearchCandidate>,
    /// Best library for the UI slot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<ResearchCandidate>,
    /// Best library for storage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<ResearchCandidate>,
    /// Best library for routing / navigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<ResearchCandidate>,
    /// Patterns / APIs the agent must NOT use (deprecated, removed, etc.).
    #[serde(default)]
    pub deprecated_blocklist: Vec<DeprecatedPattern>,
    /// Overall confidence in this report (0.0–1.0).
    pub confidence: f64,
    /// Free-form notes surfaced to the user.
    #[serde(default)]
    pub notes: Vec<String>,
}

/// A pattern the model must avoid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecatedPattern {
    /// The pattern itself (e.g. `"Provider.setState"`, `"buttonConfiguration"`).
    pub pattern: String,
    /// Why it must be avoided.
    pub reason: String,
    /// When it was deprecated, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_in: Option<String>,
    /// Suggested replacement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// Identifies a research source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSource {
    /// Official documentation site.
    OfficialDocs,
    /// pub.dev (Dart / Flutter).
    PubDev,
    /// npm (Node.js).
    Npm,
    /// crates.io (Rust).
    CratesIo,
    /// GitHub repository signals.
    Github,
    /// Stack Overflow.
    StackOverflow,
}

impl ResearchSource {
    /// All known sources. Used for the parallel runner.
    pub const ALL: &'static [ResearchSource] = &[
        Self::OfficialDocs,
        Self::PubDev,
        Self::Npm,
        Self::CratesIo,
        Self::Github,
        Self::StackOverflow,
    ];

    /// Whether this source applies to a given stack.
    pub fn supports(self, stack: Stack) -> bool {
        matches!(
            (self, stack),
            (Self::OfficialDocs, _)
                | (Self::PubDev, Stack::Flutter)
                | (
                    Self::Npm,
                    Stack::Node | Stack::React | Stack::Vue | Stack::Svelte
                )
                | (Self::CratesIo, Stack::Rust)
                | (Self::Github, _)
                | (Self::StackOverflow, _)
        )
    }

    /// Stable identifier (used for caching keys).
    pub fn id(self) -> &'static str {
        match self {
            Self::OfficialDocs => "official_docs",
            Self::PubDev => "pub.dev",
            Self::Npm => "npm",
            Self::CratesIo => "crates.io",
            Self::Github => "github",
            Self::StackOverflow => "stackoverflow",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pub_dev_supports_flutter_only() {
        assert!(ResearchSource::PubDev.supports(Stack::Flutter));
        assert!(!ResearchSource::PubDev.supports(Stack::Node));
    }

    #[test]
    fn npm_supports_node_family() {
        for s in [Stack::Node, Stack::React, Stack::Vue, Stack::Svelte] {
            assert!(ResearchSource::Npm.supports(s), "npm should support {s:?}");
        }
        assert!(!ResearchSource::Npm.supports(Stack::Python));
    }

    #[test]
    fn decision_report_round_trips() {
        let mut r = DecisionReport {
            confidence: 0.94,
            ..Default::default()
        };
        r.deprecated_blocklist.push(DeprecatedPattern {
            pattern: "Provider.setState".into(),
            reason: "deprecated in 6.1.25".into(),
            deprecated_in: Some("6.1.25".into()),
            replacement: Some("AsyncNotifier".into()),
        });
        let s = serde_json::to_string(&r).unwrap();
        let back: DecisionReport = serde_json::from_str(&s).unwrap();
        assert_eq!(back.confidence, 0.94);
        assert_eq!(back.deprecated_blocklist.len(), 1);
        assert_eq!(back.deprecated_blocklist[0].pattern, "Provider.setState");
    }
}
