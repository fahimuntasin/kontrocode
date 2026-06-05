//! Application state held by the Tauri shell.

use std::path::PathBuf;

use parking_lot::RwLock;

use kontrocode_agent::Agent;

/// Shared, thread-safe state.
pub struct AppState {
    /// The agent instance. Cheap to clone.
    pub agent: Agent,
    /// Current project root, mutable.
    project_root: RwLock<PathBuf>,
}

impl AppState {
    /// Construct a new `AppState` with the given initial project root.
    pub fn new(agent: Agent, project_root: PathBuf) -> Self {
        Self {
            agent,
            project_root: RwLock::new(project_root),
        }
    }

    /// Get the current project root.
    pub fn project_root(&self) -> PathBuf {
        self.project_root.read().clone()
    }

    /// Update the project root.
    pub fn set_project_root(&self, path: PathBuf) {
        *self.project_root.write() = path;
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("project_root", &self.project_root())
            .finish()
    }
}
