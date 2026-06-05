//! Tool implementations.
//!
//! The agent loop calls these when the model returns a tool call.
//! Each tool is a struct implementing the [`Tool`] trait. The agent
//! loop holds a registry of `Arc<dyn Tool>`s.
//!
//! Phase 1 tools: `file_read`, `file_write`, `shell_run`.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use kontrocode_core::{Error, Result, ToolCall, ToolDefinition, ToolOutput, ToolResult};
use tokio::process::Command;
use tracing::{debug, warn};

/// The trait every tool implements.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Stable, lowercase tool name. Must match the model-emitted call.
    fn name(&self) -> &'static str;

    /// One-line description for the model.
    fn description(&self) -> &'static str;

    /// JSON Schema for parameters. Should match what the model emits.
    fn parameters_schema(&self) -> serde_json::Value;

    /// The [`ToolDefinition`] for this tool, ready to send to a model.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(self.name(), self.description(), self.parameters_schema())
    }

    /// Execute the tool. `arguments` is the JSON value the model emitted.
    async fn execute(&self, arguments: serde_json::Value) -> Result<ToolOutput>;
}

/// Read a file's contents. `path` is a string. Returns text content or an
/// error message. Refuses paths outside the project root unless explicitly
/// allowed in the future.
pub struct FileReadTool {
    root: PathBuf,
}

impl FileReadTool {
    /// Construct a tool rooted at `root`. All paths are resolved against
    /// this root; absolute paths outside the root are rejected.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file. Path is relative to the project root."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file, relative to the project root."
                }
            },
            "required": ["path"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, arguments: serde_json::Value) -> Result<ToolOutput> {
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::tool("file_read", "missing required 'path' argument"))?;
        let resolved = resolve_under(&self.root, path)?;
        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| Error::filesystem(format!("{}: {e}", resolved.display())))?;
        Ok(ToolOutput::Text(content))
    }
}

/// Write a file's contents. Creates parent directories as needed. If
/// `append` is true, the content is appended instead of overwriting.
pub struct FileWriteTool {
    root: PathBuf,
}

impl FileWriteTool {
    /// Construct a tool rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &'static str {
        "file_write"
    }

    fn description(&self) -> &'static str {
        "Write content to a file. Creates parent directories. Overwrites unless 'append' is true."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
                "append": { "type": "boolean", "default": false }
            },
            "required": ["path", "content"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, arguments: serde_json::Value) -> Result<ToolOutput> {
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::tool("file_write", "missing required 'path' argument"))?;
        let content = arguments
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::tool("file_write", "missing required 'content' argument"))?;
        let append = arguments
            .get("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let resolved = resolve_under(&self.root, path)?;
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(Error::Io)?;
        }
        if append {
            use tokio::io::AsyncWriteExt;
            let mut f = tokio::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&resolved)
                .await
                .map_err(Error::Io)?;
            f.write_all(content.as_bytes()).await.map_err(Error::Io)?;
        } else {
            tokio::fs::write(&resolved, content)
                .await
                .map_err(Error::Io)?;
        }
        Ok(ToolOutput::Text(format!(
            "wrote {} bytes to {}",
            content.len(),
            resolved.display()
        )))
    }
}

/// Run a shell command. Captures stdout and stderr. Has a default timeout
/// of 30s. Destructive commands are blocked by a static blocklist.
pub struct ShellRunTool {
    root: PathBuf,
    timeout_ms: u64,
}

impl ShellRunTool {
    /// Construct a shell tool rooted at `root` with a per-call timeout
    /// in milliseconds.
    pub fn new(root: impl Into<PathBuf>, timeout_ms: u64) -> Self {
        Self {
            root: root.into(),
            timeout_ms,
        }
    }
}

#[async_trait]
impl Tool for ShellRunTool {
    fn name(&self) -> &'static str {
        "shell_run"
    }

    fn description(&self) -> &'static str {
        "Run a shell command. Returns combined stdout/stderr. Has a timeout."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "args": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "required": ["command"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, arguments: serde_json::Value) -> Result<ToolOutput> {
        let command = arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::tool("shell_run", "missing required 'command' argument"))?;
        let args: Vec<String> = arguments
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(reason) = is_blocked(command, &args) {
            return Err(Error::tool(
                "shell_run",
                format!("command blocked by safety policy: {reason}"),
            ));
        }

        debug!(command = %command, args = ?args, "shell_run");

        let mut cmd = Command::new(command);
        cmd.args(&args)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        let child = cmd
            .spawn()
            .map_err(|e| Error::shell(format!("spawn `{command}`: {e}")))?;

        let output = match tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            child.wait_with_output(),
        )
        .await
        {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(Error::shell(format!("wait: {e}"))),
            Err(_) => return Err(Error::shell("command timed out")),
        };

        let mut combined = String::new();
        if !output.stdout.is_empty() {
            combined.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        let truncated = if combined.len() > 20_000 {
            warn!(bytes = combined.len(), "shell output truncated");
            let cut = combined
                .char_indices()
                .nth(20_000)
                .map(|(i, _)| i)
                .unwrap_or(combined.len());
            let mut s = combined[..cut].to_string();
            s.push_str("\n…[truncated]");
            s
        } else {
            combined
        };

        Ok(ToolOutput::Text(format!(
            "exit={}\n{}",
            output.status.code().unwrap_or(-1),
            truncated
        )))
    }
}

/// Resolve a path against the project root, rejecting absolute paths
/// outside the root.
fn resolve_under(root: &Path, path: &str) -> Result<PathBuf> {
    let p = Path::new(path);
    let resolved = if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    };
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let resolved_canon = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
    if !resolved_canon.starts_with(&root_canon) && resolved.exists() {
        return Err(Error::filesystem(format!(
            "path `{path}` is outside the project root"
        )));
    }
    Ok(resolved)
}

/// Static safety blocklist. See SECURITY.md.
fn is_blocked(command: &str, _args: &[String]) -> Option<&'static str> {
    match command {
        "rm" if _args.iter().any(|a| a == "-rf" || a == "-fr") => Some("destructive: rm -rf"),
        "mkfs" | "mkfs.ext4" | "mkfs.ntfs" => Some("destructive: filesystem format"),
        "dd" if _args.iter().any(|a| a.starts_with("of=/dev/")) => {
            Some("destructive: dd to device")
        }
        "shutdown" | "reboot" | "halt" | "poweroff" => Some("destructive: system power"),
        "chmod"
            if _args.iter().any(|a| a == "-R" || a == "--recursive")
                && _args.iter().any(|a| a.contains("777")) =>
        {
            Some("destructive: recursive chmod 777")
        }
        _ => None,
    }
}

/// Convenience: dispatch a [`ToolCall`] to the matching tool in a list.
pub async fn dispatch(tools: &[std::sync::Arc<dyn Tool>], call: &ToolCall) -> Result<ToolResult> {
    let tool = tools
        .iter()
        .find(|t| t.name() == call.name)
        .ok_or_else(|| Error::tool(call.name.clone(), "unknown tool"))?;
    let output = tool.execute(call.arguments.clone()).await?;
    Ok(ToolResult {
        tool_call_id: call.id.clone(),
        tool_name: tool.name().to_string(),
        success: true,
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn workspace() -> TempDir {
        TempDir::new().unwrap()
    }

    #[tokio::test]
    async fn file_read_returns_contents() {
        let dir = workspace();
        std::fs::write(dir.path().join("hello.txt"), "hi there").unwrap();
        let t = FileReadTool::new(dir.path());
        let out = t
            .execute(serde_json::json!({"path": "hello.txt"}))
            .await
            .unwrap();
        match out {
            ToolOutput::Text(s) => assert_eq!(s, "hi there"),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn file_read_missing_file_errors() {
        let dir = workspace();
        let t = FileReadTool::new(dir.path());
        let err = t
            .execute(serde_json::json!({"path": "missing.txt"}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("missing.txt"));
    }

    #[tokio::test]
    async fn file_write_creates_and_overwrites() {
        let dir = workspace();
        let t = FileWriteTool::new(dir.path());
        t.execute(serde_json::json!({"path": "a/b/c.txt", "content": "v1"}))
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a/b/c.txt")).unwrap(),
            "v1"
        );
        t.execute(serde_json::json!({"path": "a/b/c.txt", "content": "v2"}))
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a/b/c.txt")).unwrap(),
            "v2"
        );
    }

    #[tokio::test]
    async fn shell_run_blocks_rm_rf() {
        let dir = workspace();
        let t = ShellRunTool::new(dir.path(), 5_000);
        let err = t
            .execute(serde_json::json!({
                "command": "rm",
                "args": ["-rf", "/"]
            }))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("blocked"));
    }

    #[tokio::test]
    async fn shell_run_executes_echo() {
        let dir = workspace();
        let t = ShellRunTool::new(dir.path(), 5_000);
        let out = t
            .execute(serde_json::json!({
                "command": "echo",
                "args": ["hello"]
            }))
            .await
            .unwrap();
        match out {
            ToolOutput::Text(s) => assert!(s.contains("hello")),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn dispatch_routes_to_correct_tool() {
        let dir = workspace();
        let tools: Vec<std::sync::Arc<dyn Tool>> = vec![
            std::sync::Arc::new(FileReadTool::new(dir.path())),
            std::sync::Arc::new(FileWriteTool::new(dir.path())),
        ];
        let call = ToolCall::new(
            "c1",
            "file_write",
            serde_json::json!({"path": "x.txt", "content": "y"}),
        );
        let res = dispatch(&tools, &call).await.unwrap();
        assert_eq!(res.tool_name, "file_write");
    }
}
