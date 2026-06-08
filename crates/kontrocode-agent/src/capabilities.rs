use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct SelfCorrector {
    max_retries: usize,
    retry_count: usize,
}

impl SelfCorrector {
    pub fn new() -> Self {
        Self { max_retries: 3, retry_count: 0 }
    }

    pub fn attempts_left(&self) -> usize {
        self.max_retries.saturating_sub(self.retry_count)
    }

    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn record_attempt(&mut self, error: &str) -> Option<String> {
        self.retry_count += 1;
        if self.retry_count < self.max_retries {
            let msg = format!("[Retry {}/{}] Compile error: {error}", self.retry_count, self.max_retries);
            info!("{msg}");
            Some(msg)
        } else {
            warn!("[FAILED] All {} retry attempts exhausted: {error}", self.max_retries);
            None
        }
    }

    pub fn run_compile_check(project_root: &PathBuf) -> Result<String, (String, usize)> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=short")
            .current_dir(project_root)
            .output();

        match output {
            Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let line = stderr.lines().last().unwrap_or("unknown error");
                Err((line.to_string(), out.status.code().unwrap_or(1) as usize))
            }
            Err(e) => Err((e.to_string(), 1)),
        }
    }

    pub fn extract_error_location(stderr: &str) -> Option<(String, usize)> {
        for line in stderr.lines() {
            if line.contains("error") && line.contains("-->") {
                let parts: Vec<&str> = line.split("-->").collect();
                if parts.len() >= 2 {
                    let loc = parts[1].trim();
                    let file_line: Vec<&str> = loc.split(':').collect();
                    if file_line.len() >= 2 {
                        let file = file_line[0].to_string();
                        let line_num = file_line[1].trim().parse::<usize>().unwrap_or(0);
                        return Some((file, line_num));
                    }
                }
            }
        }
        for line in stderr.lines() {
            if let Some(rest) = line.strip_prefix("error: ") {
                return Some((rest.to_string(), 0));
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalMode {
    Normal,
    Yolo,
    Background,
}

pub struct AgentMode {
    pub approval: ApprovalMode,
    pub max_parallel_tasks: usize,
}

impl AgentMode {
    pub fn normal() -> Self {
        Self { approval: ApprovalMode::Normal, max_parallel_tasks: 4 }
    }

    pub fn yolo() -> Self {
        Self { approval: ApprovalMode::Yolo, max_parallel_tasks: 8 }
    }

    pub fn background() -> Self {
        Self { approval: ApprovalMode::Background, max_parallel_tasks: 2 }
    }

    pub fn should_prompt_for_approval(&self) -> bool {
        matches!(self.approval, ApprovalMode::Normal)
    }
}

pub struct ContextCompactor {
    max_tokens: usize,
    keep_system: bool,
    keep_last_n: usize,
}

impl ContextCompactor {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens, keep_system: true, keep_last_n: 5 }
    }

    pub fn compact(&self, messages: &[String], system_prompt: &str) -> Vec<String> {
        let total_tokens: usize = messages.iter().map(|m| m.len() / 4).sum();
        if total_tokens <= self.max_tokens {
            return messages.to_vec();
        }

        let mut compacted: Vec<String> = Vec::new();
        if self.keep_system {
            compacted.push(format!("[system]: {system_prompt}"));
        }

        compacted.push("[Context compacted — earlier messages summarized]".into());

        let start = if messages.len() > self.keep_last_n {
            messages.len() - self.keep_last_n
        } else {
            0
        };
        for msg in &messages[start..] {
            compacted.push(msg.clone());
        }

        compacted
    }

    pub fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }
}

pub struct TestGenerator;

impl TestGenerator {
    pub fn generate_test_for(filename: &str, code: &str, language: &str) -> String {
        match language {
            "rust" => Self::generate_rust_test(filename, code),
            "python" => Self::generate_python_test(filename, code),
            "javascript" | "typescript" => Self::generate_js_test(filename, code),
            "go" => Self::generate_go_test(filename, code),
            "dart" => Self::generate_dart_test(filename, code),
            _ => format!("// TODO: add tests for {filename}\n// Language: {language}\n"),
        }
    }

    fn generate_rust_test(filename: &str, code: &str) -> String {
        let module = filename.replace('/', "::").replace(".rs", "");
        format!(
            "#[cfg(test)]\nmod tests {{\n    use super::*;\n\n    #[test]\n    fn test_{module}() {{\n        // TODO: add assertions\n        assert!(true);\n    }}\n}}\n"
        )
    }

    fn generate_python_test(filename: &str, code: &str) -> String {
        format!(
            "import pytest\n\ndef test_{}():\n    assert True\n",
            filename.replace('/', "_").replace(".py", "")
        )
    }

    fn generate_js_test(filename: &str, code: &str) -> String {
        format!(
            "describe('{f}', () => {{\n  it('should work', () => {{\n    expect(true).toBe(true);\n  }});\n}});\n",
            f = filename.replace('/', "_").replace(".ts", "").replace(".js", "")
        )
    }

    fn generate_go_test(filename: &str, code: &str) -> String {
        format!(
            "package {}\n\nimport \"testing\"\n\nfunc TestBasic(t *testing.T) {{\n\t// TODO: add assertions\n}}\n",
            filename.split('/').last().unwrap_or("main").replace(".go", "")
        )
    }

    fn generate_dart_test(filename: &str, code: &str) -> String {
        format!(
            "import 'package:flutter_test/flutter_test.dart';\n\nvoid main() {{\n  test('basic test', () {{\n    expect(true, isTrue);\n  }});\n}}\n"
        )
    }
}

pub struct ScoutSubagent {
    project_root: PathBuf,
}

impl ScoutSubagent {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn explore(&self) -> Vec<String> {
        let mut findings = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.project_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && !path.file_name().map_or(false, |n| n == "target" || n == "node_modules" || n == ".git") {
                    findings.push(format!("📁 {}/", path.file_name().unwrap_or_default().to_string_lossy()));
                } else if path.is_file() && path.extension().map_or(false, |e| e == "rs" || e == "py" || e == "ts" || e == "js" || e == "go" || e == "dart") {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    findings.push(format!("📄 {} ({} lines)", path.file_name().unwrap_or_default().to_string_lossy(), size / 40));
                }
            }
        }
        findings
    }
}
