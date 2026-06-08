use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

#[derive(Debug, Clone)]
pub struct HookResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct HookRunner {
    hooks_dir: PathBuf,
    blocked_tools: Vec<String>,
}

impl HookRunner {
    pub fn new(project_root: &Path) -> Self {
        let hooks_dir = project_root.join(".kontrocode").join("hooks");
        let blocked = Self::load_blocked_tools(&hooks_dir);
        Self {
            hooks_dir,
            blocked_tools: blocked,
        }
    }

    fn load_blocked_tools(hooks_dir: &Path) -> Vec<String> {
        let config_path = hooks_dir.join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            content
                .lines()
                .filter(|l| l.starts_with("block = "))
                .filter_map(|l| {
                    l.trim_start_matches("block = ")
                        .trim_matches('"')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .next()
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn is_blocked(&self, tool: &str) -> bool {
        self.blocked_tools.iter().any(|b| b == tool)
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
            .env("KONTROCODE_HOOKS_DIR", &self.hooks_dir)
            .output();

        match output {
            Ok(out) => {
                let passed = out.status.success();
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                if !passed {
                    info!(?tool, "hook blocked: {name}");
                }
                Some(HookResult { passed, stdout, stderr })
            }
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

    pub fn pre_research(&self) -> Option<HookResult> {
        self.run_hook("pre-research.sh", "", "")
    }

    pub fn post_research(&self) -> Option<HookResult> {
        self.run_hook("post-research.sh", "", "")
    }

    pub fn pre_shell(&self, command: &str) -> Option<HookResult> {
        self.run_hook("pre-shell.sh", "shell_run", command)
    }

    pub fn init(project_root: &Path) -> std::io::Result<()> {
        let dir = project_root.join(".kontrocode").join("hooks");
        std::fs::create_dir_all(&dir)?;

        let config_toml = dir.join("config.toml");
        if !config_toml.exists() {
            std::fs::write(&config_toml, "# Block dangerous commands\n# block = [\"rm -rf /\", \"sudo shutdown\"]\n")?;
        }

        let pre_shell = dir.join("pre-shell.sh");
        if !pre_shell.exists() {
            std::fs::write(&pre_shell, r#"#!/bin/sh
set -e
echo "About to run: $KONTROCODE_ARGS"
if echo "$KONTROCODE_ARGS" | grep -q "rm -rf /"; then
  echo "BLOCKED: destructive command"
  exit 1
fi
"#)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&pre_shell, std::fs::Permissions::from_mode(0o755))?;
            }
        }
        Ok(())
    }
}
