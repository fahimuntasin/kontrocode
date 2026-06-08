use std::collections::HashMap;
use chrono::{DateTime, Utc};
use kontrocode_core::{Fact, FactSource, Profile};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterestNode {
    pub topic: String,
    pub score: f64,
    pub related: Vec<String>,
    pub last_updated: DateTime<Utc>,
    pub hit_count: u64,
}

impl InterestNode {
    pub fn decay(&mut self) {
        let days = (Utc::now() - self.last_updated).num_hours() as f64 / 24.0;
        self.score *= 0.98f64.powf(days.max(0.0));
        self.score = self.score.clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterestGraph {
    pub nodes: HashMap<String, InterestNode>,
    pub edges: Vec<(String, String, f64)>,
}

impl InterestGraph {
    pub fn new() -> Self {
        Self { nodes: HashMap::new(), edges: Vec::new() }
    }

    pub fn from_profile(profile: &Profile) -> Self {
        let nodes: HashMap<String, InterestNode> = profile
            .interests
            .iter()
            .map(|i| {
                (i.topic.clone(), InterestNode {
                    topic: i.topic.clone(),
                    score: i.score,
                    related: Vec::new(),
                    last_updated: Utc::now(),
                    hit_count: 0,
                })
            })
            .collect();
        Self { nodes, edges: Vec::new() }
    }

    pub fn boost(&mut self, topic: &str, delta: f64) {
        let node = self.nodes.entry(topic.to_string()).or_insert_with(|| InterestNode {
            topic: topic.to_string(),
            score: 0.0,
            related: Vec::new(),
            last_updated: Utc::now(),
            hit_count: 0,
        });
        node.score = (node.score + delta).clamp(0.0, 1.0);
        node.hit_count += 1;
        node.last_updated = Utc::now();
    }

    pub fn connect(&mut self, from: &str, to: &str, weight: f64) {
        self.edges.retain(|(f, t, _)| !(f == from && t == to));
        self.edges.push((from.to_string(), to.to_string(), weight));
        if let Some(node) = self.nodes.get_mut(from) {
            if !node.related.contains(&to.to_string()) {
                node.related.push(to.to_string());
            }
        }
    }

    pub fn top_topics(&self, n: usize) -> Vec<&InterestNode> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        nodes.truncate(n);
        nodes
    }

    pub fn to_profile_summary(&self) -> String {
        let topics = self.top_topics(5);
        if topics.is_empty() {
            return "No interests yet".into();
        }
        topics.iter().map(|n| format!("{} ({:.0}%)", n.topic, n.score * 100.0)).collect::<Vec<_>>().join(", ")
    }
}

pub struct ContradictionResolver;

impl ContradictionResolver {
    pub fn resolve(new_fact: &Fact, existing: &[Fact]) -> Fact {
        let conflicting = existing.iter().find(|f| {
            f.source == FactSource::Implicit
                && text_conflicts(&new_fact.text, &f.text)
        });

        match conflicting {
            Some(old) => {
                if new_fact.confidence > old.confidence && new_fact.created_at > old.created_at {
                    info!("contradiction resolved: new fact wins (confidence {:.2} > {:.2})",
                        new_fact.confidence, old.confidence);
                    Fact {
                        id: new_fact.id.clone(),
                        text: format!("{} (overrides: {})", new_fact.text, old.text),
                        confidence: new_fact.confidence,
                        source: FactSource::Explicit,
                        ..new_fact.clone()
                    }
                } else {
                    info!("contradiction: keeping existing fact (higher confidence)");
                    Fact {
                        id: old.id.clone(),
                        text: format!("{} (contested by: {})", old.text, new_fact.text),
                        ..old.clone()
                    }
                }
            }
            None => new_fact.clone(),
        }
    }
}

fn text_conflicts(a: &str, b: &str) -> bool {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let common_keywords = ["prefer", "use", "dislike", "avoid", "reject"];
    for kw in common_keywords {
        if a_lower.contains(kw) && b_lower.contains(kw) {
            let a_rest = a_lower.split(kw).last().unwrap_or("");
            let b_rest = b_lower.split(kw).last().unwrap_or("");
            if a_rest.trim() != b_rest.trim() {
                return true;
            }
        }
    }
    false
}

pub struct ProfileVersion {
    pub version: u64,
    pub timestamp: DateTime<Utc>,
    pub snapshot: String,
    pub change_type: String,
}

pub struct ProfileVersioner {
    history: Vec<ProfileVersion>,
    base_path: String,
}

impl ProfileVersioner {
    pub fn new(base_path: &str) -> Self {
        Self { history: Vec::new(), base_path: base_path.to_string() }
    }

    pub fn save_version(&mut self, profile: &Profile, change_type: &str) -> u64 {
        let version = (self.history.len() + 1) as u64;
        let snapshot = serde_json::to_string(profile).unwrap_or_default();

        self.history.push(ProfileVersion {
            version,
            timestamp: Utc::now(),
            snapshot,
            change_type: change_type.to_string(),
        });

        if self.history.len() > 50 {
            self.history.remove(0);
        }

        version
    }

    pub fn audit_trail(&self, last_n: usize) -> String {
        let start = if self.history.len() > last_n { self.history.len() - last_n } else { 0 };
        self.history[start..]
            .iter()
            .map(|v| format!("v{} [{}] {}", v.version, v.timestamp.format("%H:%M"), v.change_type))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub struct ColdStartHandler;

impl ColdStartHandler {
    pub fn is_cold(profile: &Profile) -> bool {
        profile.facts.is_empty() && profile.stacks.is_empty() && profile.interests.is_empty()
    }

    pub fn bootstrap(profile: &mut Profile, stack_name: &str, language: &str) {
        profile.stacks.push(kontrocode_core::StackConfidence {
            name: stack_name.to_string(),
            confidence: 0.6,
            last_seen: Utc::now(),
        });
        if language == "banglish" {
            profile.preferences.language = kontrocode_core::Language::Banglish;
        }
        info!("cold start: bootstrapped profile with stack={stack_name}");
    }

    pub fn is_usable(profile: &Profile) -> bool {
        profile.facts.len() >= 2 || !profile.stacks.is_empty()
    }

    pub fn onboarding_message(profile: &Profile) -> Option<String> {
        if Self::is_cold(profile) {
            Some("👋 I'm KontroCode! I'll learn your preferences as we work. Start by asking me to build something!".into())
        } else if !Self::is_usable(profile) {
            Some(format!("🔄 Still learning... {} signals collected", profile.facts.len()))
        } else {
            None
        }
    }
}

pub struct PinnedContext {
    pub facts: Vec<Fact>,
    pub max_pins: usize,
}

impl PinnedContext {
    pub fn new() -> Self {
        Self { facts: Vec::new(), max_pins: 5 }
    }

    pub fn pin(&mut self, fact: Fact) -> Result<(), String> {
        if self.facts.len() >= self.max_pins {
            return Err(format!("Max {} pinned facts", self.max_pins));
        }
        self.facts.retain(|f| f.id != fact.id);
        self.facts.push(fact);
        Ok(())
    }

    pub fn unpin(&mut self, id: &str) {
        self.facts.retain(|f| f.id != id);
    }

    pub fn list(&self) -> &[Fact] {
        &self.facts
    }

    pub fn to_prompt_injection(&self) -> String {
        if self.facts.is_empty() {
            return String::new();
        }
        let items: Vec<String> = self.facts.iter().map(|f| format!("- {}", f.text)).collect();
        format!("<pinned_context>\n{}\n</pinned_context>\n", items.join("\n"))
    }
}

pub struct CompileFeedback;

impl CompileFeedback {
    pub fn record_success(interests: &mut InterestGraph, stack_name: &str) {
        interests.boost(stack_name, 0.05);
        info!("compile success: boosted {stack_name}");
    }

    pub fn record_failure(interests: &mut InterestGraph, error: &str) {
        let topic = extract_topic_from_error(error);
        interests.boost(&topic, -0.03);
        info!("compile failure: reduced {topic} confidence");
    }
}

fn extract_topic_from_error(error: &str) -> String {
    if error.contains("flutter") { return "Flutter".into(); }
    if error.contains("async") || error.contains("tokio") { return "Rust-Async".into(); }
    if error.contains("react") { return "React".into(); }
    "Unknown".into()
}

pub fn memory_panel_render(profile: &Profile, pins: &PinnedContext, versioner: &ProfileVersioner) -> String {
    let mut output = String::new();
    output.push_str("\n┌─ Memory Panel ────────────────────────────────┐\n");
    output.push_str(&format!("│ Facts: {} | Stacks: {} | Interests: {}       │\n",
        profile.facts.len(), profile.stacks.len(), profile.interests.len()));
    output.push_str(&format!("│ Pinned: {}                                    │\n", pins.facts.len()));

    for fact in profile.facts.iter().take(5) {
        let conf = (fact.confidence * 100.0) as usize;
        output.push_str(&format!("│ [{conf}%] {:.45} │\n", fact.text));
    }

    if profile.facts.len() > 5 {
        output.push_str(&format!("│ ... and {} more facts                         │\n", profile.facts.len() - 5));
    }

    output.push_str("│                                               │\n");
    output.push_str(&format!("│ Audit trail:                                  │\n"));
    for line in versioner.audit_trail(3).lines() {
        output.push_str(&format!("│ {:<45} │\n", line));
    }

    output.push_str("└─────────────────────────────────────────────────┘\n");
    output
}
