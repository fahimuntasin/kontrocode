use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTool {
    pub name: String,
    pub provider_id: String,
    pub status: ToolStatus,
    pub endpoint: String,
    pub models: Vec<String>,
    pub one_click: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolStatus { Connected, NotInstalled, Error(String) }

impl DetectedTool {
    fn ok(name: &str, id: &str, endpoint: &str, models: Vec<String>) -> Self {
        Self { name: name.into(), provider_id: id.into(), status: ToolStatus::Connected, endpoint: endpoint.into(), models, one_click: true }
    }
    fn missing(name: &str, id: &str) -> Self {
        Self { name: name.into(), provider_id: id.into(), status: ToolStatus::NotInstalled, endpoint: String::new(), models: vec![], one_click: false }
    }
}

pub struct AutoConnector {
    pub tools: Vec<DetectedTool>,
}

impl AutoConnector {
    pub fn scan() -> Self {
        let mut tools = Vec::new();
        tools.push(Self::detect_ollama());
        tools.push(Self::detect_opencode());
        tools.push(Self::detect_claude_code());
        tools.push(Self::detect_gemini_cli());
        tools.push(Self::detect_lmstudio());
        tools.push(Self::detect_copilot_cli());
        tools.push(Self::detect_continue());
        tools.push(Self::detect_gpt4all());
        Self { tools }
    }

    fn detect_ollama() -> DetectedTool {
        match Command::new("ollama").arg("list").output() {
            Ok(out) if out.status.success() => {
                let models: Vec<String> = String::from_utf8_lossy(&out.stdout)
                    .lines().skip(1).filter_map(|l| l.split_whitespace().next().map(|s| s.to_string())).collect();
                DetectedTool::ok("Ollama", "sonic", "http://localhost:11434/v1", models)
            }
            _ => DetectedTool::missing("Ollama", "sonic"),
        }
    }

    fn detect_opencode() -> DetectedTool {
        match Command::new("opencode").arg("--version").output() {
            Ok(out) if out.status.success() => 
                DetectedTool::ok("OpenCode", "opencode", "opencode", vec!["claude-sonnet-4".into(), "gpt-4o".into()]),
            _ => DetectedTool::missing("OpenCode", "opencode"),
        }
    }

    fn detect_claude_code() -> DetectedTool {
        match Command::new("claude").arg("--version").output() {
            Ok(out) if out.status.success() => 
                DetectedTool::ok("Claude Code", "claude-code", "claude", vec!["claude-sonnet-4".into(), "claude-opus-4-5".into()]),
            _ => DetectedTool::missing("Claude Code", "claude-code"),
        }
    }

    fn detect_gemini_cli() -> DetectedTool {
        match Command::new("gemini").arg("--version").output() {
            Ok(out) if out.status.success() => 
                DetectedTool::ok("Gemini CLI", "gemini-cli", "gemini", vec!["gemini-2.5-flash".into(), "gemini-2.5-pro".into()]),
            _ => DetectedTool::missing("Gemini CLI", "gemini-cli"),
        }
    }

    fn detect_lmstudio() -> DetectedTool {
        match curl("http://localhost:1234/v1/models") {
            true => DetectedTool::ok("LM Studio", "lmstudio", "http://localhost:1234/v1", vec!["local-model".into()]),
            false => DetectedTool::missing("LM Studio", "lmstudio"),
        }
    }

    fn detect_copilot_cli() -> DetectedTool {
        match Command::new("gh").arg("copilot").arg("--version").output() {
            Ok(out) if out.status.success() => DetectedTool::ok("GitHub Copilot", "copilot", "gh copilot", vec!["copilot".into()]),
            _ => DetectedTool::missing("GitHub Copilot", "copilot"),
        }
    }

    fn detect_continue() -> DetectedTool {
        let home = std::env::var("HOME").unwrap_or_default();
        if std::path::Path::new(&format!("{home}/.continue/continue.json")).exists()
            || std::path::Path::new(".continue/continue.json").exists() {
            DetectedTool::ok("Continue.dev", "continue", "continue", vec!["codestral".into(), "gpt-4o-mini".into()])
        } else {
            DetectedTool::missing("Continue.dev", "continue")
        }
    }

    fn detect_gpt4all() -> DetectedTool {
        match curl("http://localhost:4891/v1/models") {
            true => DetectedTool::ok("GPT4All", "gpt4all", "http://localhost:4891/v1", vec!["local-model".into()]),
            false => DetectedTool::missing("GPT4All", "gpt4all"),
        }
    }

    pub fn connected_count(&self) -> usize {
        self.tools.iter().filter(|t| t.status == ToolStatus::Connected).count()
    }

    pub fn render_dashboard(&self) -> String {
        let mut output = String::new();
        output.push_str("\n┌─ Auto-Connect Dashboard ────────────────────┐\n");
        for tool in &self.tools {
            let icon = match tool.status { ToolStatus::Connected => "✅", ToolStatus::NotInstalled => "⬜", ToolStatus::Error(_) => "⚠️" };
            let cnt = if tool.models.is_empty() { "none".to_string() } else { format!("{} models", tool.models.len()) };
            output.push_str(&format!("│ {icon} {:<18} {:<12}         │\n", tool.name, cnt));
        }
        output.push_str(&format!("│ Connected: {}/{}                              │\n", self.connected_count(), self.tools.len()));
        output.push_str("└──────────────────────────────────────────────┘\n");
        output
    }

    pub fn render_one_click(&self) -> String {
        let connected: Vec<_> = self.tools.iter().filter(|t| t.status == ToolStatus::Connected).collect();
        let missing: Vec<_> = self.tools.iter().filter(|t| t.status == ToolStatus::NotInstalled).collect();
        let mut out = String::from("## One-Click Connect\n\n### ✅ Connected\n");
        for t in &connected {
            out.push_str(&format!("- **{name}** → `{id}` ({models})\n", name=t.name, id=t.provider_id, models=t.models.first().map(|s|s.as_str()).unwrap_or("detected")));
        }
        if !missing.is_empty() {
            out.push_str("\n### 📦 Available (not installed)\n");
            for t in &missing { out.push_str(&format!("- {} — install to auto-connect\n", t.name)); }
        }
        out
    }
}

fn curl(url: &str) -> bool {
    matches!(Command::new("curl").args(["-s","-m","2",url]).output(), Ok(o) if o.status.success())
}
