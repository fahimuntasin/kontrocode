//! Model scoring — given a task, rank candidate models.
//!
//! Phase 1 ships the cost-optimized scorer. The speed and quality scorers
//! are stubbed with the same structure but use different weights. They
//! land fully in Phase 2 alongside the real providers.

use kontrocode_core::ModelSpec;

/// How complex the task is. Drives model selection.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum TaskComplexity {
    /// Trivial — pattern match, lookup, simple edit. Cheap model.
    Trivial,
    /// Routine — single-file edit, well-defined task. Cheap-to-mid model.
    Routine,
    /// Multi-file or architecturally significant. Mid model.
    Complex,
    /// Cross-system design or novel problem. Top-tier model.
    Expert,
}

impl TaskComplexity {
    /// Construct from a `0.0..=1.0` score.
    pub fn from_score(s: f64) -> Self {
        if s < 0.25 {
            Self::Trivial
        } else if s < 0.55 {
            Self::Routine
        } else if s < 0.8 {
            Self::Complex
        } else {
            Self::Expert
        }
    }
}

/// A model + its computed score for a task.
#[derive(Debug, Clone)]
pub struct ScoredModel {
    /// The candidate model spec.
    pub spec: ModelSpec,
    /// Composite score in `0.0..=1.0`. Higher is better.
    pub score: f64,
}

/// What we care about when picking a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringCriteria {
    /// Minimize cost for a given quality floor.
    Cost,
    /// Minimize latency for a given quality floor.
    Speed,
    /// Maximize quality regardless of cost.
    Quality,
}

/// Score every candidate against the criteria and complexity, then sort
/// descending by score.
pub fn rank(
    models: &[ModelSpec],
    criteria: ScoringCriteria,
    complexity: TaskComplexity,
) -> Vec<ScoredModel> {
    let mut out: Vec<ScoredModel> = models
        .iter()
        .map(|spec| ScoredModel {
            spec: spec.clone(),
            score: score_one(spec, criteria, complexity),
        })
        .collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn score_one(spec: &ModelSpec, criteria: ScoringCriteria, complexity: TaskComplexity) -> f64 {
    let quality = quality_floor(spec, complexity);
    match criteria {
        ScoringCriteria::Cost => {
            // Reward low price, reward meeting quality floor.
            let price = spec.input_price_per_mtok + spec.output_price_per_mtok;
            let price_score = 1.0 / (1.0 + price / 10.0);
            0.7 * price_score + 0.3 * quality
        }
        ScoringCriteria::Speed => {
            // We don't have latency metadata in Phase 1; proxy with
            // smaller context window being faster.
            let size_score = 1.0 - ((spec.context_window as f64).log10() / 7.0).min(1.0);
            0.6 * size_score + 0.4 * quality
        }
        ScoringCriteria::Quality => quality,
    }
}

fn quality_floor(spec: &ModelSpec, complexity: TaskComplexity) -> f64 {
    // Crude proxy: bigger context window + higher max output = stronger model.
    let ctx_score = (spec.context_window as f64 / 200_000.0).min(1.0);
    let out_score = (spec.max_output_tokens as f64 / 16_000.0).min(1.0);
    let capability = (ctx_score + out_score) / 2.0;
    let required = match complexity {
        TaskComplexity::Trivial => 0.3,
        TaskComplexity::Routine => 0.5,
        TaskComplexity::Complex => 0.75,
        TaskComplexity::Expert => 0.9,
    };
    if capability >= required {
        1.0
    } else {
        capability / required * 0.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kontrocode_core::ModelId;

    fn cheap_model() -> ModelSpec {
        ModelSpec {
            id: ModelId::new("mock", "cheap"),
            display_name: "Cheap".into(),
            context_window: 32_000,
            max_output_tokens: 4_096,
            input_price_per_mtok: 0.10,
            output_price_per_mtok: 0.30,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: false,
        }
    }

    fn big_model() -> ModelSpec {
        ModelSpec {
            id: ModelId::new("mock", "opus"),
            display_name: "Opus".into(),
            context_window: 200_000,
            max_output_tokens: 16_000,
            input_price_per_mtok: 15.0,
            output_price_per_mtok: 75.0,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
        }
    }

    #[test]
    fn complexity_thresholds() {
        assert_eq!(TaskComplexity::from_score(0.1), TaskComplexity::Trivial);
        assert_eq!(TaskComplexity::from_score(0.4), TaskComplexity::Routine);
        assert_eq!(TaskComplexity::from_score(0.7), TaskComplexity::Complex);
        assert_eq!(TaskComplexity::from_score(0.95), TaskComplexity::Expert);
    }

    #[test]
    fn cost_mode_prefers_cheap_for_simple_tasks() {
        let ranked = rank(
            &[big_model(), cheap_model()],
            ScoringCriteria::Cost,
            TaskComplexity::Trivial,
        );
        assert_eq!(ranked[0].spec.id, cheap_model().id);
    }

    #[test]
    fn quality_mode_prefers_big_for_expert_tasks() {
        let ranked = rank(
            &[cheap_model(), big_model()],
            ScoringCriteria::Quality,
            TaskComplexity::Expert,
        );
        assert_eq!(ranked[0].spec.id, big_model().id);
    }

    #[test]
    fn cheap_fails_quality_floor_for_expert() {
        let ranked = rank(
            &[cheap_model()],
            ScoringCriteria::Cost,
            TaskComplexity::Expert,
        );
        // Score should be reduced because cheap can't meet expert floor.
        assert!(ranked[0].score < 1.0);
    }
}
