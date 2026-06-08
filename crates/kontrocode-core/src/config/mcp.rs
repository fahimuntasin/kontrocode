use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRegistry {
    pub servers: HashMap<String, McpServerConfig>,
}

impl McpRegistry {
    pub fn load_bundled() -> Self {
        let json = include_str!("../../../../config/mcp_servers.json");
        serde_json::from_str(json).unwrap_or_else(|_| McpRegistry {
            servers: HashMap::new(),
        })
    }

    pub fn get(&self, id: &str) -> Option<&McpServerConfig> {
        self.servers.get(id)
    }

    pub fn list(&self) -> Vec<&String> {
        self.servers.keys().collect()
    }

    pub fn resolve_env(&self, config: &McpServerConfig) -> HashMap<String, String> {
        let mut resolved = HashMap::new();
        for (key, value) in &config.env {
            let resolved_value = if value.starts_with("${") && value.ends_with('}') {
                let env_var = &value[2..value.len() - 1];
                std::env::var(env_var).unwrap_or_else(|_| value.clone())
            } else {
                value.clone()
            };
            resolved.insert(key.clone(), resolved_value);
        }
        resolved
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub name: String,
    pub description: String,
    pub stack: String,
    pub category: String,
    pub featured: bool,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub mcp_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRegistry {
    pub templates: HashMap<String, TemplateConfig>,
}

impl TemplateRegistry {
    pub fn load_bundled() -> Self {
        let json = include_str!("../../../../config/templates.json");
        serde_json::from_str(json).unwrap_or_else(|_| TemplateRegistry {
            templates: HashMap::new(),
        })
    }

    pub fn featured(&self) -> Vec<&TemplateConfig> {
        self.templates.values().filter(|t| t.featured).collect()
    }

    pub fn by_stack(&self, stack: &str) -> Vec<&TemplateConfig> {
        self.templates
            .values()
            .filter(|t| t.stack == stack)
            .collect()
    }

    pub fn by_category(&self, category: &str) -> Vec<&TemplateConfig> {
        self.templates
            .values()
            .filter(|t| t.category == category)
            .collect()
    }
}
