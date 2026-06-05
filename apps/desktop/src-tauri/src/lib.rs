//! Tauri v2 shell for KontroCode.
//!
//! Wires the UI to the Rust agent loop. Tauri commands (`agent_*`,
//! `file_*`, `shell_*`, `memory_*`) bridge the Solid.js frontend to the
//! `kontrocode-agent` crate over typed IPC. Streaming agent output is
//! emitted as Tauri events.
//!
//! See `apps/desktop/src-tauri/tauri.conf.json` for the window config
//! and `apps/desktop/src-tauri/capabilities/default.json` for the
//! permission model.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use kontrocode_agent::tools::Tool;
use kontrocode_agent::{Agent, AgentConfig, StreamSender};
use kontrocode_core::{Message, Profile};
use kontrocode_memory::{FileMemoryStore, MemoryStore};
use kontrocode_research::{NullSource, ResearchRunner, ResearchRunnerConfig};
use kontrocode_router::{MockProvider, ProviderRegistry, Router};

mod state;
mod tools;
mod tray;

use state::AppState;

/// Application entry point invoked by `main.rs`.
pub fn run() {
    // Initialize tracing. Honours RUST_LOG.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kontrocode=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();

    info!("KontroCode desktop v{} starting", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // Project root for the initial session. UI can change this
            // via the `set_project_root` command.
            let initial_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

            // Build the agent.
            let memory: Arc<dyn MemoryStore> = Arc::new(FileMemoryStore::default_location());
            let router = Router::with_default_config(ProviderRegistry::new().tap(|r| {
                r.register(MockProvider::new());
            }));
            let runner =
                ResearchRunner::new(vec![Arc::new(NullSource)], ResearchRunnerConfig::default());
            let agent_config = AgentConfig::new(&initial_root);
            let tools = tools::default_tools(&initial_root);
            let agent = Agent::new(agent_config, router, runner, memory, tools);

            let state = AppState::new(agent, initial_root);
            app.manage(state);

            // Optional: install a system tray (Phase 1 best-effort).
            #[cfg(desktop)]
            {
                if let Err(e) = tray::install(app.handle()) {
                    warn!(error = %e, "failed to install system tray (non-fatal)");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_agent_send,
            cmd_agent_history,
            cmd_agent_cancel,
            cmd_memory_get_profile,
            cmd_memory_update_fact,
            cmd_memory_delete_fact,
            cmd_file_read,
            cmd_file_write,
            cmd_shell_run,
            cmd_set_project_root,
            cmd_get_app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Send a user message to the agent. Returns the stream subscription id.
/// All agent events are emitted to the UI as Tauri events named
/// `agent:event:<subscription_id>`.
#[tauri::command]
async fn cmd_agent_send(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<String, String> {
    let (_tx, rx, handle) = state.agent.submit(text);
    let subscription_id = uuid::Uuid::new_v4().to_string();

    // Convert the receiver into a futures::Stream and forward every
    // agent event to the UI as a Tauri event.
    let mut stream = rx.into_stream();
    let app_handle = app.clone();
    let sub = subscription_id.clone();
    tauri::async_runtime::spawn(async move {
        use futures::StreamExt;
        let event_name = format!("agent:event:{sub}");
        while let Some(event) = stream.next().await {
            if let Err(e) = app_handle.emit(&event_name, &event) {
                warn!(error = %e, subscription = %sub, "failed to emit agent event");
            }
        }
        if let Err(e) = handle.await {
            warn!(error = %e, "agent task join error");
        }
    });

    Ok(subscription_id)
}

/// Return the conversation history (in-memory, Phase 1: empty).
#[tauri::command]
async fn cmd_agent_history(_state: State<'_, AppState>) -> Result<Vec<Message>, String> {
    // Phase 1: no persistent history. Phase 4 adds it.
    Ok(Vec::new())
}

/// Cancel an in-flight agent run (Phase 1: no-op).
#[tauri::command]
async fn cmd_agent_cancel(
    _state: State<'_, AppState>,
    _subscription_id: String,
) -> Result<(), String> {
    // Phase 1: cancellation token plumbing is a Phase 4 task.
    Ok(())
}

/// Read the current user profile.
#[tauri::command]
async fn cmd_memory_get_profile(state: State<'_, AppState>) -> Result<Profile, String> {
    state.agent.memory().load().await.map_err(|e| e.to_string())
}

/// Update a fact's text.
#[tauri::command]
async fn cmd_memory_update_fact(
    state: State<'_, AppState>,
    id: String,
    text: String,
) -> Result<(), String> {
    state
        .agent
        .memory()
        .update_fact(&id, text)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a fact.
#[tauri::command]
async fn cmd_memory_delete_fact(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .agent
        .memory()
        .delete_fact(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Read a file (rooted at the project root).
#[tauri::command]
async fn cmd_file_read(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let tool = kontrocode_agent::tools::FileReadTool::new(state.project_root());
    let out = tool
        .execute(serde_json::json!({"path": path}))
        .await
        .map_err(|e| e.to_string())?;
    match out {
        kontrocode_core::ToolOutput::Text(s) => Ok(s),
        kontrocode_core::ToolOutput::Json(v) => Ok(v.to_string()),
    }
}

/// Write a file (rooted at the project root).
#[tauri::command]
async fn cmd_file_write(
    state: State<'_, AppState>,
    path: String,
    content: String,
) -> Result<String, String> {
    let tool = kontrocode_agent::tools::FileWriteTool::new(state.project_root());
    let out = tool
        .execute(serde_json::json!({"path": path, "content": content}))
        .await
        .map_err(|e| e.to_string())?;
    match out {
        kontrocode_core::ToolOutput::Text(s) => Ok(s),
        kontrocode_core::ToolOutput::Json(v) => Ok(v.to_string()),
    }
}

/// Run a shell command (rooted at the project root).
#[tauri::command]
async fn cmd_shell_run(
    state: State<'_, AppState>,
    command: String,
    args: Vec<String>,
) -> Result<String, String> {
    let tool = kontrocode_agent::tools::ShellRunTool::new(state.project_root(), 30_000);
    let out = tool
        .execute(serde_json::json!({"command": command, "args": args}))
        .await
        .map_err(|e| e.to_string())?;
    match out {
        kontrocode_core::ToolOutput::Text(s) => Ok(s),
        kontrocode_core::ToolOutput::Json(v) => Ok(v.to_string()),
    }
}

/// Update the project root. Rebuilds the file tools on the agent.
#[tauri::command]
async fn cmd_set_project_root(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("path does not exist: {path}"));
    }
    if !p.is_dir() {
        return Err(format!("path is not a directory: {path}"));
    }
    state.set_project_root(p);
    Ok(())
}

/// Information about the running app. Displayed in the status bar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// KontroCode version.
    pub version: String,
    /// The Tauri runtime version.
    pub tauri_version: String,
    /// The current project root.
    pub project_root: String,
    /// The default model id.
    pub default_model: String,
    /// Number of registered providers.
    pub provider_count: usize,
    /// Whether research is enabled.
    pub research_enabled: bool,
}

/// Return runtime information for the status bar.
#[tauri::command]
async fn cmd_get_app_info(state: State<'_, AppState>) -> Result<AppInfo, String> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        project_root: state.project_root().display().to_string(),
        default_model: state.agent.default_model().to_string(),
        provider_count: state.agent.registry_len(),
        research_enabled: true,
    })
}

/// Helper trait for `tap`-style mutation on a value.
trait Tap: Sized {
    /// Run `f` on `self`, then return `self`.
    fn tap<F: FnOnce(&mut Self)>(mut self, f: F) -> Self {
        f(&mut self);
        self
    }
}

impl<T> Tap for T {}

// We don't use StreamSender in the command module itself; it's exposed
// for future use. Re-export under an alias to make the intent clear.
#[allow(dead_code)]
type AgentStreamSender = StreamSender;
