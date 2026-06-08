use std::fmt;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct AgentCell {
    pub id: String,
    pub name: String,
    pub task: String,
    pub status: CellStatus,
    pub progress: f64,
    pub tokens_used: usize,
    pub file: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl fmt::Display for CellStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let emoji = match self {
            CellStatus::Queued => "⏳",
            CellStatus::Running => "⚡",
            CellStatus::Completed => "✅",
            CellStatus::Failed => "❌",
        };
        write!(f, "{emoji}")
    }
}

pub struct MissionControl {
    grid: Vec<AgentCell>,
    elapsed: Instant,
    event_log: Vec<String>,
}

impl MissionControl {
    pub fn new() -> Self {
        Self { grid: Vec::new(), elapsed: Instant::now(), event_log: Vec::new() }
    }

    pub fn add_agent(&mut self, cell: AgentCell) {
        self.event_log.push(format!("{} Agent {}: {}", cell.status, cell.name, cell.task));
        self.grid.push(cell);
    }

    pub fn update_agent(&mut self, id: &str, new_status: CellStatus, progress: f64) {
        for cell in &mut self.grid {
            if cell.id == id {
                cell.status = new_status.clone();
                cell.progress = progress;
            }
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("\n═══ KONTROCODE MISSION CONTROL ═══\n");
        output.push_str(&format!("Agents: {} | Uptime: {}s\n", self.grid.len(), self.elapsed.elapsed().as_secs()));
        output.push_str("────────────────────────────\n");
        for cell in &self.grid {
            let bar = progress_bar(cell.progress, 20);
            output.push_str(&format!("{} {:<12} {} {}%\n", cell.status, cell.name, bar, (cell.progress * 100.0) as usize));
        }
        output.push_str("════════════════════════════\n");
        output
    }

    pub fn render_compact(&self) -> String {
        let mut lines = vec![format!("MC: {} agents", self.grid.len())];
        for cell in &self.grid {
            let pct = (cell.progress * 100.0) as usize;
            lines.push(format!("  {} {} {}% {}", cell.status, cell.name, pct, cell.task));
        }
        lines.join("\n")
    }

    pub fn total_agents(&self) -> usize { self.grid.len() }
}

fn progress_bar(pct: f64, width: usize) -> String {
    let filled = (pct * width as f64) as usize;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(width - filled))
}
