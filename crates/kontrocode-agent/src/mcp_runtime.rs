use std::collections::HashMap;
use std::process::Stdio;

use kontrocode_core::config::mcp::{McpRegistry, McpServerConfig};
use tokio::process::Command;
use tracing::{debug, info, warn};

pub struct McpManager {
    registry: McpRegistry,
    active: HashMap<String, tokio::process::Child>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            registry: McpRegistry::load_bundled(),
            active: HashMap::new(),
        }
    }

    pub fn servers(&self) -> Vec<(&String, &McpServerConfig)> {
        self.registry.servers.iter().collect()
    }

    pub async fn connect(&mut self, server_id: &str) -> Result<(), String> {
        if self.active.contains_key(server_id) {
            return Ok(());
        }

        let config = self
            .registry
            .get(server_id)
            .ok_or_else(|| format!("unknown MCP server: {server_id}"))?;

        let env = self.registry.resolve_env(config);
        info!("mcp: starting {} ({})", config.name, config.command);

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &env {
            cmd.env(key, value);
        }

        match cmd.spawn() {
            Ok(child) => {
                self.active.insert(server_id.to_string(), child);
                info!("mcp: {} connected", config.name);
                Ok(())
            }
            Err(e) => {
                warn!("mcp: {} failed to start: {e}", config.name);
                Err(e.to_string())
            }
        }
    }

    pub async fn connect_all(&mut self) {
        let ids: Vec<String> = self.registry.servers.keys().cloned().collect();
        for id in &ids {
            let _ = self.connect(id).await;
        }
    }

    pub async fn disconnect(&mut self, server_id: &str) {
        if let Some(mut child) = self.active.remove(server_id) {
            let _ = child.kill().await;
            info!("mcp: {server_id} disconnected");
        }
    }

    pub async fn shutdown(&mut self) {
        let ids: Vec<String> = self.active.keys().cloned().collect();
        for id in ids {
            self.disconnect(&id).await;
        }
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

impl Drop for McpManager {
    fn drop(&mut self) {
        for (_, mut child) in self.active.drain() {
            let _ = child.start_kill();
        }
    }
}

pub fn list_available_mcps() -> Vec<String> {
    McpRegistry::load_bundled()
        .servers
        .keys()
        .cloned()
        .collect()
}
