use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub messages: Vec<SessionMessage>,
    pub stack: Option<String>,
    pub files_touched: Vec<String>,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

pub struct SessionHistory {
    sessions: VecDeque<SessionRecord>,
    max_sessions: usize,
    storage_path: PathBuf,
}

impl SessionHistory {
    pub fn new(storage_path: PathBuf) -> Self {
        let mut h = Self { sessions: VecDeque::new(), max_sessions: 100, storage_path };
        h.load();
        h
    }

    pub fn add(&mut self, s: SessionRecord) {
        self.sessions.push_front(s);
        while self.sessions.len() > self.max_sessions { self.sessions.pop_back(); }
        self.save();
    }

    pub fn search(&self, q: &str) -> Vec<&SessionRecord> {
        let q = q.to_lowercase();
        self.sessions.iter().filter(|s| {
            s.messages.iter().any(|m| m.content.to_lowercase().contains(&q))
                || s.stack.as_ref().map_or(false, |st| st.to_lowercase().contains(&q))
                || s.files_touched.iter().any(|f| f.to_lowercase().contains(&q))
        }).collect()
    }

    pub fn len(&self) -> usize { self.sessions.len() }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string(&self.sessions) {
            fs::write(&self.storage_path, json).ok();
        }
    }

    fn load(&mut self) {
        if let Ok(json) = fs::read_to_string(&self.storage_path) {
            if let Ok(sessions) = serde_json::from_str(&json) {
                self.sessions = sessions;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ResponseLanguage { English, Banglish, Bengali }

impl ResponseLanguage {
    pub fn translate(&self, key: &str) -> String {
        match (self, key) {
            (Self::Banglish, "welcome") => "👋 Welcome! Ami KontroCode. Ki korbo bolen?",
            (Self::Bengali, "welcome") => "👋 স্বাগতম! আমি KontroCode। কী করবো বলুন?",
            _ => key,
        }.to_string()
    }
}

pub struct Onboarding;
impl Onboarding {
    pub fn flow(lang: ResponseLanguage, count: usize) -> Vec<String> {
        let mut msgs = vec![lang.translate("welcome")];
        if count < 5 { msgs.push(format!("🔄 Learning... {} / 5", count + 1)); }
        else { msgs.push("✅ Full profile ready!".into()); }
        msgs
    }
}

pub struct OfflineMode {
    pub local_model: bool,
}
impl OfflineMode {
    pub fn new() -> Self {
        let ollama = std::process::Command::new("ollama").arg("list").output().map(|o| o.status.success()).unwrap_or(false);
        Self { local_model: ollama }
    }
    pub fn can_work(&self) -> bool { self.local_model }
}

pub struct JiraClient {
    pub configured: bool,
}
impl JiraClient {
    pub fn from_env() -> Self {
        Self { configured: std::env::var("JIRA_BASE_URL").is_ok() && std::env::var("JIRA_API_TOKEN").is_ok() }
    }
}
