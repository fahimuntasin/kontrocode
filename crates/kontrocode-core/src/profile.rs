//! Persistent user profile — what the agent knows about you.
//!
//! The profile is the heart of the memory system. It is built passively
//! from signal observation (Facebook-style) and never asked of the user
//! directly. See PRD §4.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The full user profile, persisted in the memory store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Stable user id. For Phase 1 this is the OS username hashed with
    /// the installation id; Phase 7 ties it to an account.
    pub user_id: Uuid,
    /// Compressed 2–3 sentence description of the user.
    pub summary: String,
    /// Communication preferences.
    pub preferences: Preferences,
    /// Stacks the user is known to work in.
    pub stacks: Vec<StackConfidence>,
    /// Discrete facts the agent has learned.
    pub facts: Vec<Fact>,
    /// Topics of interest with decaying scores.
    pub interests: Vec<Interest>,
    /// Last update timestamp (UTC).
    pub last_updated: DateTime<Utc>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            summary: String::new(),
            preferences: Preferences::default(),
            stacks: Vec::new(),
            facts: Vec::new(),
            interests: Vec::new(),
            last_updated: Utc::now(),
        }
    }
}

/// User communication preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    /// Preferred response verbosity.
    pub response_style: ResponseStyle,
    /// Preferred language for prose.
    pub language: Language,
    /// Self-reported expertise on a 0.0–1.0 scale. Drives explanation depth.
    pub expertise_level: ExpertiseLevel,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            response_style: ResponseStyle::Balanced,
            language: Language::English,
            expertise_level: ExpertiseLevel(0.5),
        }
    }
}

/// How verbose the agent should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStyle {
    /// Short, direct, minimal explanation.
    Concise,
    /// Balanced — explain non-obvious decisions.
    Balanced,
    /// Thorough — explain the why behind every step.
    Detailed,
}

/// Language for prose responses. Code/identifiers always remain English.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Romanized Bengali (e.g. "ki holo?").
    Banglish,
    /// English.
    English,
    /// Bengali script.
    Bengali,
}

/// Expertise level on a 0.0–1.0 scale. Newtype for clarity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExpertiseLevel(pub f64);

impl ExpertiseLevel {
    /// Construct a clamped expertise level.
    pub fn new(v: f64) -> Self {
        Self(v.clamp(0.0, 1.0))
    }

    /// Returns `true` if the user is considered expert (`> 0.7`).
    pub fn is_expert(&self) -> bool {
        self.0 > 0.7
    }

    /// Returns `true` if the user is considered beginner (`< 0.4`).
    pub fn is_beginner(&self) -> bool {
        self.0 < 0.4
    }
}

impl Default for ExpertiseLevel {
    fn default() -> Self {
        Self(0.5)
    }
}

/// A technology stack the user is known to work in, with confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackConfidence {
    /// Stack name.
    pub name: String,
    /// Confidence in `0.0..=1.0`. Decays over time.
    pub confidence: f64,
    /// Last time we saw this stack in action.
    pub last_seen: DateTime<Utc>,
}

/// A discrete fact the agent has learned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Stable id.
    pub id: String,
    /// The fact, in natural language.
    pub text: String,
    /// Confidence in `0.0..=1.0`.
    pub confidence: f64,
    /// When this fact was first recorded.
    pub created_at: DateTime<Utc>,
    /// Where this fact came from.
    pub source: FactSource,
    /// Semantic embedding for RAG search. None if not yet computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<crate::embedding::Embedding>,
}

/// Provenance of a [`Fact`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactSource {
    /// User said it directly. Highest confidence.
    Explicit,
    /// Detected from user behavior.
    Implicit,
    /// Inferred by the model.
    Inferred,
    /// Imported from another source.
    Imported,
}

/// A topic of interest with a decaying score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interest {
    /// Topic name (e.g. "Flutter", "AI infra").
    pub topic: String,
    /// Score in `0.0..=1.0`.
    pub score: f64,
    /// Daily decay rate.
    pub decay_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expertise_level_clamps() {
        assert_eq!(ExpertiseLevel::new(1.5).0, 1.0);
        assert_eq!(ExpertiseLevel::new(-0.5).0, 0.0);
        assert_eq!(ExpertiseLevel::new(0.5).0, 0.5);
    }

    #[test]
    fn expertise_thresholds() {
        assert!(!ExpertiseLevel::new(0.5).is_expert());
        assert!(ExpertiseLevel::new(0.8).is_expert());
        assert!(ExpertiseLevel::new(0.3).is_beginner());
    }

    #[test]
    fn profile_default_is_empty_but_valid() {
        let p = Profile::default();
        assert!(p.facts.is_empty());
        assert!(p.stacks.is_empty());
    }

    #[test]
    fn profile_round_trips_through_json() {
        let p = Profile {
            user_id: Uuid::new_v4(),
            summary: "Flutter dev".into(),
            preferences: Preferences {
                response_style: ResponseStyle::Concise,
                language: Language::Banglish,
                expertise_level: ExpertiseLevel(0.8),
            },
            stacks: vec![StackConfidence {
                name: "Flutter".into(),
                confidence: 0.95,
                last_seen: Utc::now(),
            }],
            facts: vec![Fact {
                id: "f1".into(),
                text: "prefers Riverpod over Provider".into(),
                confidence: 0.9,
                created_at: Utc::now(),
                source: FactSource::Implicit,
                embedding: None,
            }],
            interests: vec![],
            last_updated: Utc::now(),
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: Profile = serde_json::from_str(&s).unwrap();
        assert_eq!(back.summary, "Flutter dev");
        assert_eq!(back.facts.len(), 1);
        assert_eq!(back.preferences.language, Language::Banglish);
    }
}
