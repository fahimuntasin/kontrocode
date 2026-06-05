//! Exponential decay for interest scores.
//!
//! Per PRD §4.3: `interest.score *= 0.98` per day of no reinforcement.
//! The actual decay rate is configurable; this module is the single place
//! that applies it.

use chrono::{DateTime, Duration, Utc};
use kontrocode_core::{Interest, Profile};

/// Apply decay to all interests based on elapsed time since `last_updated`.
///
/// `decay_rate` is the per-day multiplier. Default: `0.02` (i.e. score
/// is multiplied by `(1 - 0.02)^days`).
pub fn apply_decay(profile: &mut Profile, now: DateTime<Utc>, default_rate: f64) {
    let elapsed = now.signed_duration_since(profile.last_updated);
    let days = (elapsed.num_seconds() as f64 / 86_400.0).max(0.0);
    if days < 0.001 {
        return;
    }
    for interest in &mut profile.interests {
        let rate = if interest.decay_rate > 0.0 {
            interest.decay_rate
        } else {
            default_rate
        };
        let factor = (1.0 - rate).powf(days);
        interest.score = (interest.score * factor).clamp(0.0, 1.0);
    }
    profile.last_updated = now;
}

/// Reinforce a single interest — bump its score and reset decay clock.
pub fn reinforce(interest: &mut Interest, delta: f64) {
    interest.score = (interest.score + delta).clamp(0.0, 1.0);
}

/// A simple helper for tests: `days_ago(7)` returns `Utc::now() - 7 days`.
pub fn days_ago(n: i64) -> DateTime<Utc> {
    Utc::now() - Duration::days(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontrocode_core::Interest;

    fn profile_with_interests(scores: &[f64]) -> Profile {
        Profile {
            interests: scores
                .iter()
                .enumerate()
                .map(|(i, s)| Interest {
                    topic: format!("topic-{i}"),
                    score: *s,
                    decay_rate: 0.0, // use default
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn decay_reduces_scores_over_time() {
        let mut p = profile_with_interests(&[1.0, 0.5]);
        p.last_updated = Utc::now() - Duration::days(10);
        apply_decay(&mut p, Utc::now(), 0.02);
        assert!(p.interests[0].score < 1.0);
        assert!(p.interests[1].score < 0.5);
    }

    #[test]
    fn decay_respects_per_interest_rate() {
        let mut p = Profile::default();
        p.interests.push(Interest {
            topic: "fast".into(),
            score: 1.0,
            decay_rate: 0.5,
        });
        p.last_updated = Utc::now() - Duration::days(1);
        apply_decay(&mut p, Utc::now(), 0.02);
        // 0.5^1 = 0.5
        assert!((p.interests[0].score - 0.5).abs() < 0.01);
    }

    #[test]
    fn decay_is_noop_within_one_second() {
        let mut p = profile_with_interests(&[0.8]);
        apply_decay(&mut p, Utc::now(), 0.02);
        assert!((p.interests[0].score - 0.8).abs() < 1e-9);
    }

    #[test]
    fn reinforce_bumps_score() {
        let mut i = Interest {
            topic: "x".into(),
            score: 0.5,
            decay_rate: 0.0,
        };
        reinforce(&mut i, 0.3);
        assert!((i.score - 0.8).abs() < 1e-9);
    }
}
