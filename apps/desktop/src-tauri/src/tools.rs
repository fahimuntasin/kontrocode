//! Default tool set installed on the agent.

use std::path::Path;
use std::sync::Arc;

use kontrocode_agent::tools::{FileReadTool, FileWriteTool, ShellRunTool, Tool};

/// Build the default tool set for a project rooted at `root`.
pub fn default_tools(root: &Path) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(FileReadTool::new(root)),
        Arc::new(FileWriteTool::new(root)),
        Arc::new(ShellRunTool::new(root, 30_000)),
    ]
}
