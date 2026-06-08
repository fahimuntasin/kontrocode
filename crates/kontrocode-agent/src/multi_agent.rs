use std::sync::Arc;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: String,
    pub description: String,
    pub file_path: Option<String>,
    pub task_type: SubtaskType,
    pub dependencies: Vec<String>,
    pub status: SubtaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubtaskType {
    Research,
    CodeGeneration,
    FileWrite,
    ShellRun,
    Validation,
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubtaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskResult {
    pub subtask_id: String,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentPlan {
    pub request: String,
    pub subtasks: Vec<Subtask>,
    pub estimated_steps: usize,
}

pub struct TaskPlanner;

impl TaskPlanner {
    pub fn plan(request: &str) -> MultiAgentPlan {
        let request_lower = request.to_lowercase();
        let mut subtasks = Vec::new();
        let mut id_counter = 0;

        let mut next_id = || {
            id_counter += 1;
            format!("task-{id_counter}")
        };

        let research_id = next_id();
        subtasks.push(Subtask {
            id: research_id.clone(),
            description: format!("Research: find best approach for '{request}'"),
            file_path: None,
            task_type: SubtaskType::Research,
            dependencies: vec![],
            status: SubtaskStatus::Pending,
        });

        if request_lower.contains("create") || request_lower.contains("build") || request_lower.contains("generate") {
            let gen_id = next_id();
            subtasks.push(Subtask {
                id: gen_id.clone(),
                description: "Generate implementation code".into(),
                file_path: None,
                task_type: SubtaskType::CodeGeneration,
                dependencies: vec![research_id.clone()],
                status: SubtaskStatus::Pending,
            });

            let write_id = next_id();
            subtasks.push(Subtask {
                id: write_id.clone(),
                description: "Write generated code to files".into(),
                file_path: None,
                task_type: SubtaskType::FileWrite,
                dependencies: vec![gen_id],
                status: SubtaskStatus::Pending,
            });

            let validate_id = next_id();
            subtasks.push(Subtask {
                id: validate_id,
                description: "Validate: check imports, syntax, and build".into(),
                file_path: None,
                task_type: SubtaskType::Validation,
                dependencies: vec![write_id],
                status: SubtaskStatus::Pending,
            });
        }

        if request_lower.contains("fix") || request_lower.contains("bug") {
            subtasks.push(Subtask {
                id: next_id(),
                description: "Analyze bug and identify root cause".into(),
                file_path: None,
                task_type: SubtaskType::Research,
                dependencies: vec![],
                status: SubtaskStatus::Pending,
            });
            subtasks.push(Subtask {
                id: next_id(),
                description: "Apply fix with minimal diff".into(),
                file_path: None,
                task_type: SubtaskType::CodeGeneration,
                dependencies: vec![research_id],
                status: SubtaskStatus::Pending,
            });
        }

        MultiAgentPlan {
            request: request.to_string(),
            estimated_steps: subtasks.len(),
            subtasks,
        }
    }

    pub fn execution_order(plan: &MultiAgentPlan) -> Vec<Vec<usize>> {
        let mut remaining: Vec<usize> = (0..plan.subtasks.len()).collect();
        let mut batches: Vec<Vec<usize>> = Vec::new();

        while !remaining.is_empty() {
            let mut batch: Vec<usize> = Vec::new();
            remaining.retain(|&i| {
                let deps_met = plan.subtasks[i]
                    .dependencies
                    .iter()
                    .all(|dep_id| {
                        plan.subtasks.iter().any(|s| &s.id == dep_id && s.status == SubtaskStatus::Completed)
                    });
                if deps_met || plan.subtasks[i].dependencies.is_empty() {
                    batch.push(i);
                    false
                } else {
                    true
                }
            });

            if batch.is_empty() && !remaining.is_empty() {
                break;
            }
            batches.push(batch);
        }

        batches
    }
}

pub struct MultiAgentRunner {
    plan: MultiAgentPlan,
    results: Vec<SubtaskResult>,
}

impl MultiAgentRunner {
    pub fn new(request: &str) -> Self {
        let plan = TaskPlanner::plan(request);
        info!(
            "multi-agent plan: {} subtasks for '{}'",
            plan.subtasks.len(),
            request
        );
        Self {
            plan,
            results: Vec::new(),
        }
    }

    pub fn subtask_count(&self) -> usize {
        self.plan.subtasks.len()
    }

    pub fn plan_summary(&self) -> String {
        let mut summary = format!(
            "## Multi-Agent Plan\n\n**Request:** {}\n\n",
            self.plan.request
        );
        for (i, task) in self.plan.subtasks.iter().enumerate() {
            summary.push_str(&format!(
                "{}. **{}** [{:?}] — {}\n",
                i + 1,
                task.id,
                task.task_type,
                task.description
            ));
        }
        summary
    }

    pub async fn run_parallel<F, Fut>(&mut self, executor: F) -> Vec<SubtaskResult>
    where
        F: Fn(&Subtask) -> Fut,
        Fut: std::future::Future<Output = SubtaskResult>,
    {
        let batches = TaskPlanner::execution_order(&self.plan);

        for batch in batches {
            let mut futures = Vec::new();
            for &idx in &batch {
                let subtask = &self.plan.subtasks[idx];
                futures.push(executor(subtask));
            }

            let batch_results = join_all(futures).await;
            for result in batch_results {
                self.plan.subtasks.iter_mut().for_each(|s| {
                    if s.id == result.subtask_id {
                        s.status = if result.success {
                            SubtaskStatus::Completed
                        } else {
                            SubtaskStatus::Failed(result.output.clone())
                        };
                    }
                });
                self.results.push(result);
            }
        }

        self.results.clone()
    }
}
