//! Implicit signal collection (Facebook-style).
//!
//! The agent never asks the user to fill a profile. Instead, every
//! interaction emits a [`Signal`] that the memory subsystem processes.
//! See PRD §4.1 for the full list of signal kinds and their weights.

use chrono::{DateTime, Utc};
use kontrocode_core::Profile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::decay::reinforce;

/// What the user just did. Every event in the UI is one of these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Unique signal id.
    pub id: Uuid,
    /// What happened.
    pub kind: SignalKind,
    /// Topic this signal is about (e.g. `"Flutter"`, `"rust-async"`).
    /// `None` for non-topic signals.
    pub topic: Option<String>,
    /// When the signal occurred.
    pub at: DateTime<Utc>,
    /// Free-form metadata (e.g. library name, file path).
    pub meta: serde_json::Value,
}

/// The kind of signal observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    /// User copied a code block. Strong positive for the topic.
    CodeBlockCopied,
    /// User heavily edited the agent's output. Negative signal.
    HeavyEdit,
    /// User asked to regenerate. Strong negative — rethink approach.
    RegenerateRequest,
    /// User asked a follow-up — count this for "follow-up depth".
    FollowUp,
    /// Long dwell on a response without next message. Strong engagement.
    LongDwell,
    /// User replaced a suggested library. Record actual preference.
    LibraryReplaced,
    /// User mentioned a stack/library by name. Highest confidence.
    ExplicitStackMention,
    /// Same error appeared twice. Knowledge gap.
    RecurringError,
    /// User said something in a particular language. Style signal.
    LanguageSample,
    /// Session start / end. Time-of-day signal.
    SessionBoundary,
}

impl SignalKind {
    /// Default reinforcement magnitude for a topic interest.
    pub fn topic_delta(self) -> Option<f64> {
        match self {
            Self::CodeBlockCopied => Some(0.08),
            Self::HeavyEdit => Some(-0.05),
            Self::RegenerateRequest => Some(-0.15),
            Self::FollowUp => Some(0.03),
            Self::LongDwell => Some(0.05),
            Self::LibraryReplaced => None, // handled differently
            Self::ExplicitStackMention => Some(0.20),
            Self::RecurringError => None, // handled differently
            Self::LanguageSample => None, // handled differently
            Self::SessionBoundary => None,
        }
    }
}

/// Apply a single signal to a profile, mutating it in place.
pub fn apply_signal(profile: &mut Profile, signal: &Signal) {
    if let Some(topic) = &signal.topic {
        if let Some(delta) = signal.kind.topic_delta() {
            if let Some(interest) = profile.interests.iter_mut().find(|i| &i.topic == topic) {
                reinforce(interest, delta);
            } else {
                profile.interests.push(kontrocode_core::Interest {
                    topic: topic.clone(),
                    score: 0.5 + delta,
                    decay_rate: 0.0,
                });
            }
        }
    }

    match signal.kind {
        SignalKind::LibraryReplaced => {
            if let Some(replacement) = signal.meta.get("replacement").and_then(|v| v.as_str()) {
                if let Some(stack) = signal.topic.as_deref() {
                    profile
                        .stacks
                        .retain(|s| s.name != format!("{stack} (replaced)"));
                    profile.stacks.push(kontrocode_core::StackConfidence {
                        name: replacement.to_string(),
                        confidence: 0.9,
                        last_seen: Utc::now(),
                    });
                }
            }
        }
        SignalKind::ExplicitStackMention => {
            if let Some(name) = signal.meta.get("name").and_then(|v| v.as_str()) {
                if let Some(s) = profile.stacks.iter_mut().find(|s| s.name == name) {
                    s.confidence = (s.confidence + 0.1).min(1.0);
                    s.last_seen = Utc::now();
                } else {
                    profile.stacks.push(kontrocode_core::StackConfidence {
                        name: name.to_string(),
                        confidence: 0.85,
                        last_seen: Utc::now(),
                    });
                }
            }
        }
        SignalKind::LanguageSample => {
            if let Some(lang) = signal.meta.get("language").and_then(|v| v.as_str()) {
                let parsed = match lang {
                    "banglish" => kontrocode_core::Language::Banglish,
                    "english" => kontrocode_core::Language::English,
                    "bengali" => kontrocode_core::Language::Bengali,
                    _ => return,
                };
                profile.preferences.language = parsed;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontrocode_core::FactSource;
    use serde_json::json;

    fn signal(kind: SignalKind, topic: Option<&str>, meta: serde_json::Value) -> Signal {
        Signal {
            id: Uuid::new_v4(),
            kind,
            topic: topic.map(String::from),
            at: Utc::now(),
            meta,
        }
    }

    #[test]
    fn code_copy_reinforces_topic() {
        let mut p = Profile::default();
        apply_signal(
            &mut p,
            &signal(SignalKind::CodeBlockCopied, Some("Flutter"), json!({})),
        );
        let i = p.interests.iter().find(|i| i.topic == "Flutter").unwrap();
        assert!(i.score > 0.5);
    }

    #[test]
    fn regenerate_negatively_reinforces() {
        let mut p = Profile::default();
        apply_signal(
            &mut p,
            &signal(SignalKind::RegenerateRequest, Some("Rust"), json!({})),
        );
        let i = p.interests.iter().find(|i| i.topic == "Rust").unwrap();
        assert!(i.score < 0.5);
    }

    #[test]
    fn explicit_stack_mention_adds_stack_with_high_confidence() {
        let mut p = Profile::default();
        apply_signal(
            &mut p,
            &signal(
                SignalKind::ExplicitStackMention,
                None,
                json!({"name": "Flutter"}),
            ),
        );
        let s = p.stacks.iter().find(|s| s.name == "Flutter").unwrap();
        assert!(s.confidence >= 0.85);
    }

    #[test]
    fn library_replacement_records_preference() {
        let mut p = Profile::default();
        apply_signal(
            &mut p,
            &signal(
                SignalKind::LibraryReplaced,
                Some("State"),
                json!({"replacement": "Riverpod"}),
            ),
        );
        let s = p.stacks.iter().find(|s| s.name == "Riverpod").unwrap();
        assert!(s.confidence >= 0.85);
    }

    #[test]
    fn language_sample_sets_preference() {
        let mut p = Profile::default();
        apply_signal(
            &mut p,
            &signal(
                SignalKind::LanguageSample,
                None,
                json!({"language": "banglish"}),
            ),
        );
        assert_eq!(p.preferences.language, kontrocode_core::Language::Banglish);
    }

    #[test]
    fn explicit_fact_can_be_added_via_helper() {
        let mut p = Profile::default();
        p.facts.push(kontrocode_core::Fact {
            id: "f1".into(),
            text: "prefers Riverpod".into(),
            confidence: 0.9,
            created_at: Utc::now(),
            source: FactSource::Explicit,
        });
        assert_eq!(p.facts.len(), 1);
    }
}
