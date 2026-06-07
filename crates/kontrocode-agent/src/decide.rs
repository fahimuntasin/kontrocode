use kontrocode_core::ResearchCandidate;

pub struct Decision {
    pub winner: ResearchCandidate,
    pub confidence: f64,
    pub alternatives: Vec<ResearchCandidate>,
    pub reasoning: String,
}

pub fn decide(mut candidates: Vec<ResearchCandidate>) -> Option<Decision> {
    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let winner = candidates.remove(0);
    let confidence = winner.score;

    let reasoning = format!(
        "{} ({:.1}) — score={:.2} from {}",
        winner.name, winner.version, winner.score, winner.source
    );

    Some(Decision {
        winner,
        confidence,
        alternatives: candidates,
        reasoning,
    })
}

pub fn validate_deprecation(candidates: &[ResearchCandidate]) -> Vec<String> {
    candidates
        .iter()
        .filter(|c| c.score < 0.1 || c.reason.contains("deprecated"))
        .map(|c| format!("{} — {:.2} ({})", c.name, c.score, c.reason))
        .collect()
}

pub fn validate_version_conflict(
    deps: &[(String, String)],
) -> Vec<String> {
    let mut warnings = Vec::new();
    for (name, version) in deps {
        if version.starts_with('0') {
            warnings.push(format!("{name}@{version} is pre-1.0, breaking changes may occur"));
        }
    }
    warnings
}
