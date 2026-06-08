use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct HookResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct HookRunner {
    hooks_dir: PathBuf,
}

impl HookRunner {
    pub fn new(project_root: &Path) -> Self {
        Self {
            hooks_dir: project_root.join(".kontrocode").join("hooks"),
        }
    }

    fn run_hook(&self, name: &str, tool: &str, args: &str) -> Option<HookResult> {
        let hook_path = self.hooks_dir.join(name);
        if !hook_path.exists() || !hook_path.is_file() {
            return None;
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(&hook_path)
            .env("KONTROCODE_TOOL", tool)
            .env("KONTROCODE_ARGS", args)
            .output();

        match output {
            Ok(out) => Some(HookResult {
                passed: out.status.success(),
                stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            }),
            Err(e) => Some(HookResult {
                passed: false,
                stdout: String::new(),
                stderr: format!("hook error: {e}"),
            }),
        }
    }

    pub fn pre_tool(&self, tool: &str, args: &str) -> Option<HookResult> {
        self.run_hook("pre-tool.sh", tool, args)
    }

    pub fn post_tool(&self, tool: &str, args: &str) -> Option<HookResult> {
        self.run_hook("post-tool.sh", tool, args)
    }

    pub fn pre_generate(&self) -> Option<HookResult> {
        self.run_hook("pre-generate.sh", "", "")
    }

    pub fn post_generate(&self) -> Option<HookResult> {
        self.run_hook("post-generate.sh", "", "")
    }
}
