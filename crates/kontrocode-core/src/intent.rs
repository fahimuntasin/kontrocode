//! User intent analysis — what is the user actually asking for?
//!
//! The intent analyzer runs on every user message before research and
//! routing. It is intentionally simple in Phase 1: stack detection,
//! task-type classification, and a complexity score. Smarter intent
//! analysis comes with the planner in later phases.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Detected intent for a user message.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Intent {
    /// Detected stacks (e.g. `["Flutter", "Dart"]`). Always sorted, deduped.
    pub stacks: Vec<Stack>,
    /// Detected task type.
    pub task_type: TaskType,
    /// Complexity score in `0.0..=1.0`. Drives model routing.
    pub complexity: f64,
    /// Whether external research is needed (new libraries, API usage,
    /// current best practices). Pure refactors in known code: `false`.
    pub needs_research: bool,
    /// Free-form, human-readable summary of what the user wants.
    pub summary: String,
}

impl Intent {
    /// Returns `true` if any stacks were detected.
    pub fn has_stacks(&self) -> bool {
        !self.stacks.is_empty()
    }
}

/// A technology stack detected from the user message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stack {
    /// Flutter / Dart.
    Flutter,
    /// React / Next.js.
    React,
    /// Vue / Nuxt.
    Vue,
    /// Svelte / SvelteKit.
    Svelte,
    /// Node.js / Deno / Bun (server JS/TS).
    Node,
    /// Python.
    Python,
    /// Rust.
    Rust,
    /// Go.
    Go,
    /// Swift / iOS.
    Swift,
    /// Kotlin / Android.
    Kotlin,
    /// Generic "no specific stack" — unknown or general purpose.
    Unknown,
}

impl std::fmt::Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

impl Stack {
    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Flutter => "Flutter",
            Self::React => "React",
            Self::Vue => "Vue",
            Self::Svelte => "Svelte",
            Self::Node => "Node.js",
            Self::Python => "Python",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Swift => "Swift",
            Self::Kotlin => "Kotlin",
            Self::Unknown => "Unknown",
        }
    }

    /// Try to detect a stack from a token in the user message.
    pub fn detect(token: &str) -> Option<Self> {
        let t = token.to_ascii_lowercase();
        match t.as_str() {
            "flutter" | "dart" => Some(Self::Flutter),
            "react" | "next" | "nextjs" | "next.js" => Some(Self::React),
            "vue" | "nuxt" => Some(Self::Vue),
            "svelte" | "sveltekit" => Some(Self::Svelte),
            "node" | "nodejs" | "node.js" | "deno" | "bun" | "ts" | "typescript" | "js"
            | "javascript" => Some(Self::Node),
            "python" | "py" | "django" | "flask" | "fastapi" => Some(Self::Python),
            "rust" | "rs" => Some(Self::Rust),
            "go" | "golang" => Some(Self::Go),
            "swift" | "ios" => Some(Self::Swift),
            "kotlin" | "android" => Some(Self::Kotlin),
            _ => None,
        }
    }
}

/// Coarse task classification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Build a new feature or screen from scratch.
    Build,
    /// Modify existing code.
    Modify,
    /// Refactor without changing behavior.
    Refactor,
    /// Diagnose a bug or error.
    Debug,
    /// Explain code.
    Explain,
    /// Add or run tests.
    Test,
    /// Run a shell command or git operation.
    Shell,
    /// Could not classify.
    #[default]
    Other,
}

/// Heuristic intent analyzer. In Phase 1 this is purely lexical; the
/// agent loop will replace it with a model-driven call when the model
/// router is wired in.
pub fn analyze(prompt: &str) -> Intent {
    let lower = prompt.to_ascii_lowercase();
    let tokens: BTreeSet<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '.')
        .filter(|s| !s.is_empty())
        .collect();

    let mut stacks: Vec<Stack> = tokens
        .iter()
        .filter_map(|t| Stack::detect(t))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if stacks.is_empty() {
        stacks.push(Stack::Unknown);
    }

    let task_type = classify_task(&lower);
    let complexity = score_complexity(prompt, &stacks, task_type);
    let needs_research = needs_research(&lower, &stacks);

    Intent {
        stacks,
        task_type,
        complexity,
        needs_research,
        summary: summarize(prompt),
    }
}

fn classify_task(lower: &str) -> TaskType {
    if lower.starts_with("refactor") || lower.contains(" refactor ") {
        TaskType::Refactor
    } else if lower.starts_with("fix")
        || lower.contains(" error ")
        || lower.contains(" bug ")
        || lower.contains(" not working")
        || lower.contains("doesn't work")
    {
        TaskType::Debug
    } else if lower.starts_with("explain")
        || lower.starts_with("what does")
        || lower.starts_with("how does")
    {
        TaskType::Explain
    } else if lower.contains("write a test")
        || lower.contains("add tests")
        || lower.contains("unit test")
    {
        TaskType::Test
    } else if lower.starts_with("run ") || lower.starts_with("execute ") || lower.contains("git ") {
        TaskType::Shell
    } else if lower.starts_with("modify ")
        || lower.starts_with("update ")
        || lower.starts_with("change ")
        || lower.starts_with("edit ")
    {
        TaskType::Modify
    } else {
        TaskType::Build
    }
}

fn score_complexity(prompt: &str, stacks: &[Stack], task_type: TaskType) -> f64 {
    let word_count = prompt.split_whitespace().count();
    let stack_boost = if stacks.iter().any(|s| *s != Stack::Unknown) {
        0.1
    } else {
        0.0
    };
    let task_boost = match task_type {
        TaskType::Build => 0.2,
        TaskType::Debug => 0.25,
        TaskType::Refactor => 0.3,
        TaskType::Test => 0.15,
        TaskType::Explain => 0.1,
        TaskType::Modify => 0.2,
        TaskType::Shell => 0.05,
        TaskType::Other => 0.1,
    };
    let length_score = (word_count as f64 / 80.0).clamp(0.0, 0.4);
    (0.2 + length_score + task_boost + stack_boost).clamp(0.0, 1.0)
}

fn needs_research(lower: &str, stacks: &[Stack]) -> bool {
    if lower.contains("latest") || lower.contains("current") || lower.contains("best practice") {
        return true;
    }
    if lower.contains("package") || lower.contains("library") || lower.contains("api") {
        return true;
    }
    if lower.contains("deprecated") {
        return true;
    }
    stacks.iter().any(|s| *s != Stack::Unknown)
}

fn summarize(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.len() <= 120 {
        trimmed.to_string()
    } else {
        let mut end = 120;
        while !trimmed.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &trimmed[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_flutter_stack() {
        let i = analyze("build me a Flutter auth screen with Google Sign-In");
        assert!(i.stacks.contains(&Stack::Flutter));
        assert_eq!(i.task_type, TaskType::Build);
        assert!(i.needs_research);
    }

    #[test]
    fn unknown_stack_when_no_keywords() {
        let i = analyze("organize my files");
        assert_eq!(i.stacks, vec![Stack::Unknown]);
    }

    #[test]
    fn debug_task_detected() {
        let i = analyze("fix the TypeError in my Python script");
        assert_eq!(i.task_type, TaskType::Debug);
        assert!(i.stacks.contains(&Stack::Python));
    }

    #[test]
    fn refactor_task_detected() {
        let i = analyze("refactor this class to use composition");
        assert_eq!(i.task_type, TaskType::Refactor);
    }

    #[test]
    fn complexity_in_unit_range() {
        let i = analyze("build me a Flutter auth screen with Google Sign-In");
        assert!((0.0..=1.0).contains(&i.complexity));
    }
}
