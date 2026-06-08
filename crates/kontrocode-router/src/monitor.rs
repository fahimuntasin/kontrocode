use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct LatencyRecord {
    pub model: String,
    pub latency_ms: u64,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
    pub tokens_input: u32,
    pub tokens_output: u32,
}

pub struct HealthMonitor {
    records: Arc<RwLock<Vec<LatencyRecord>>>,
    provider_status: Arc<RwLock<HashMap<String, bool>>>,
    total_cost: Arc<RwLock<f64>>,
    budget_cap: Arc<RwLock<Option<f64>>>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
            provider_status: Arc::new(RwLock::new(HashMap::new())),
            total_cost: Arc::new(RwLock::new(0.0)),
            budget_cap: Arc::new(RwLock::new(None)),
        }
    }

    pub fn record(&self, record: LatencyRecord) {
        self.records.write().push(record.clone());
        self.provider_status.write().insert(record.model.clone(), record.success);
        *self.total_cost.write() += (record.tokens_input as f64 / 1_000_000.0) * 2.5
            + (record.tokens_output as f64 / 1_000_000.0) * 10.0;
    }

    pub fn p95_latency(&self, model: &str) -> u64 {
        let records = self.records.read();
        let mut lats: Vec<u64> = records.iter()
            .filter(|r| r.model == model && r.success)
            .map(|r| r.latency_ms)
            .collect();
        if lats.is_empty() { return 0; }
        lats.sort();
        let idx = (lats.len() as f64 * 0.95) as usize;
        lats[lats.len().min(idx).saturating_sub(1)]
    }

    pub fn avg_latency(&self, model: &str) -> u64 {
        let records = self.records.read();
        let lats: Vec<u64> = records.iter()
            .filter(|r| r.model == model)
            .map(|r| r.latency_ms)
            .collect();
        if lats.is_empty() { return 0; }
        (lats.iter().sum::<u64>() / lats.len() as u64)
    }

    pub fn is_healthy(&self, model: &str) -> bool {
        *self.provider_status.read().get(model).unwrap_or(&true)
    }

    pub fn total_cost(&self) -> f64 {
        *self.total_cost.read()
    }

    pub fn set_budget_cap(&self, cap: Option<f64>) {
        *self.budget_cap.write() = cap;
    }

    pub fn is_over_budget(&self) -> bool {
        match *self.budget_cap.read() {
            Some(cap) => self.total_cost() >= cap,
            None => false,
        }
    }

    pub fn export_csv(&self) -> String {
        let mut csv = String::from("timestamp,model,latency_ms,success,tokens_in,tokens_out\n");
        for r in self.records.read().iter() {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                r.timestamp.format("%Y-%m-%dT%H:%M:%S"),
                r.model,
                r.latency_ms,
                r.success,
                r.tokens_input,
                r.tokens_output,
            ));
        }
        csv
    }

    pub fn dashboard(&self) -> String {
        let mut output = String::new();
        output.push_str("\n┌─ Provider Dashboard ───────────────────────────┐\n");
        output.push_str(&format!("│ Total cost: ${:.4}                            │\n", self.total_cost()));

        if let Some(cap) = *self.budget_cap.read() {
            let pct = (self.total_cost() / cap * 100.0).min(100.0);
            let bar = "█".repeat((pct / 5.0) as usize) + &"░".repeat(20 - (pct / 5.0) as usize);
            output.push_str(&format!("│ Budget: ${:.2}/{cap:.2} {} {:.0}%             │\n",
                self.total_cost(), bar, pct));
        }

        let records = self.records.read();
        let mut models: HashMap<String, (u64, usize)> = HashMap::new();
        for r in records.iter() {
            let entry = models.entry(r.model.clone()).or_insert((0, 0));
            entry.0 += r.latency_ms;
            entry.1 += 1;
        }

        for (model, (total_lat, count)) in &models {
            let avg = total_lat / *count as u64;
            let healthy = self.is_healthy(model);
            let icon = if healthy { "✓" } else { "✗" };
            output.push_str(&format!("│ {icon} {model:<15} avg {avg}ms ({count} reqs)  │\n"));
        }

        output.push_str("└─────────────────────────────────────────────────┘\n");
        output
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoutingMode {
    CostOptimized,
    SpeedOptimized,
    QualityOptimized,
    Balanced,
}

pub struct CostTracker {
    model_costs: HashMap<String, f64>,
    total_spent: f64,
}

impl CostTracker {
    pub fn new() -> Self {
        Self { model_costs: HashMap::new(), total_spent: 0.0 }
    }

    pub fn add_usage(&mut self, model: &str, input_tokens: u32, output_tokens: u32) {
        let cost = (input_tokens as f64 / 1_000_000.0) * 2.5
            + (output_tokens as f64 / 1_000_000.0) * 10.0;
        *self.model_costs.entry(model.to_string()).or_default() += cost;
        self.total_spent += cost;
    }

    pub fn estimate(&self, model: &str, estimated_tokens: u32) -> f64 {
        (estimated_tokens as f64 / 1_000_000.0) * 2.5
    }

    pub fn cheapest_model(&self) -> Option<&str> {
        self.model_costs.iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(m, _)| m.as_str())
    }

    pub fn to_csv(&self) -> String {
        let mut csv = String::from("model,total_cost_usd\n");
        for (model, cost) in &self.model_costs {
            csv.push_str(&format!("{model},{cost:.4}\n"));
        }
        csv
    }
}
