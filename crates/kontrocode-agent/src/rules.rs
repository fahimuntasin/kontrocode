use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct Rule {
    pub path: PathBuf,
    pub content: String,
    pub rule_type: RuleType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleType {
    Always,
    AgentRequested,
    Manual,
}

impl RuleType {
    pub fn from_path(path: &Path) -> Self {
        let parent = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str());
        match parent {
            Some("always") => RuleType::Always,
            Some("agent_requested") => RuleType::AgentRequested,
            _ => RuleType::Manual,
        }
    }
}

pub fn load_rules(project_root: &Path) -> Vec<Rule> {
    let rules_dir = project_root.join(".kontrocode").join("rules");
    if !rules_dir.exists() {
        return Vec::new();
    }
    let mut rules = Vec::new();
    if let Ok(entries) = fs::read_dir(&rules_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Ok(content) = fs::read_to_string(&path) {
                    rules.push(Rule {
                        rule_type: RuleType::from_path(&path),
                        path: path.clone(),
                        content,
                    });
                }
            }
            if path.is_dir() {
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    for sub in sub_entries.flatten() {
                        let sub_path = sub.path();
                        if sub_path.extension().and_then(|e| e.to_str()) == Some("md") {
                            if let Ok(content) = fs::read_to_string(&sub_path) {
                                rules.push(Rule {
                                    rule_type: RuleType::from_path(&sub_path),
                                    path: sub_path.clone(),
                                    content,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    rules
}

pub fn build_rules_prompt(rules: &[Rule]) -> String {
    if rules.is_empty() {
        return String::new();
    }
    let always: Vec<_> = rules.iter().filter(|r| r.rule_type == RuleType::Always).collect();
    let requested: Vec<_> = rules.iter().filter(|r| r.rule_type == RuleType::AgentRequested).collect();

    let mut prompt = String::from("<rules>\n");
    prompt.push_str("The rules section has a number of possible rules/memories/context that you should consider. In each subsection, we provide instructions about what information the subsection contains and how you should consider/follow the contents of the subsection.\n\n");

    if !always.is_empty() {
        prompt.push_str("<always_applied_workspace_rules description=\"These are rules set by the project that you should follow if appropriate.\">\n");
        for rule in &always {
            prompt.push_str(&rule.content);
            prompt.push('\n');
        }
        prompt.push_str("</always_applied_workspace_rules>\n\n");
    }

    if !requested.is_empty() {
        prompt.push_str("<agent_requestable_workspace_rules description=\"These are workspace-level rules that the agent should follow. They can request the full details of the rule with the read_rules tool.\">\n");
        for rule in &requested {
            let name = rule.path.file_stem().and_then(|n| n.to_str()).unwrap_or("unknown");
            prompt.push_str(&format!("Use read rule tool to fetch the rule content if needed. In <agent_requestable_workspace_rules> section, key is rule's path, value is rule's description.\n- {name}: {}\n", 
                rule.content.lines().next().unwrap_or(&rule.content)));
        }
        prompt.push_str("</agent_requestable_workspace_rules>\n\n");
    }

    prompt.push_str("</rules>\n");
    prompt
}

pub fn init_rules(project_root: &Path) -> std::io::Result<()> {
    let rules_dir = project_root.join(".kontrocode").join("rules").join("always");
    fs::create_dir_all(&rules_dir)?;
    let readme = rules_dir.join("kontrocode.md");
    if !readme.exists() {
        fs::write(&readme, r#"# KontroCode Project Rules

These rules are automatically injected into every agent session.
Edit them to guide the agent's behavior for this project.

## Example rules:
# - Always use functional components over class components
# - Prefer async/await over raw promises
# - Use TypeScript strict mode for all new files
# - Commit messages follow conventional commits format
"#)?;
    }
    Ok(())
}
