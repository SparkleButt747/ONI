//! Plan persistence — saves/loads the current orchestrator plan to disk.
//! Allows resuming multi-step tasks across sessions.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedPlan {
    /// Human-readable task description
    pub task: String,
    /// All steps in the plan
    pub steps: Vec<PlanStep>,
    /// Project directory this plan applies to
    pub project_dir: String,
    /// Unix timestamp when plan was created
    pub created_at: u64,
    /// Unix timestamp of last update
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub index: usize,
    pub description: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Done,
    Failed,
    Skipped,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::InProgress => write!(f, "IN_PROGRESS"),
            Self::Done => write!(f, "DONE"),
            Self::Failed => write!(f, "FAILED"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}

fn plan_path(project_dir: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    project_dir.hash(&mut hasher);
    let hash = hasher.finish();
    let data = oni_core::config::data_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
    data.join(format!("active-plan-{:x}.json", hash))
}

impl PersistedPlan {
    /// Load the active plan from disk for the given project directory. Returns None if no plan exists.
    pub fn load(project_dir: &str) -> Option<Self> {
        let path = plan_path(project_dir);
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    /// Save the plan to disk.
    pub fn save(&self) {
        let path = plan_path(&self.project_dir);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, serde_json::to_string_pretty(self).unwrap_or_default());
    }

    /// Clear the active plan for the given project directory.
    pub fn clear(project_dir: &str) {
        let path = plan_path(project_dir);
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Create a new plan from task description and steps.
    pub fn new(task: &str, steps: Vec<String>, project_dir: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            task: task.to_string(),
            steps: steps
                .into_iter()
                .enumerate()
                .map(|(i, desc)| PlanStep {
                    index: i + 1,
                    description: desc,
                    status: StepStatus::Pending,
                })
                .collect(),
            project_dir: project_dir.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Mark a step as done.
    pub fn complete_step(&mut self, index: usize) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.index == index) {
            step.status = StepStatus::Done;
            self.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.save();
        }
    }

    /// Mark a step as in progress.
    pub fn start_step(&mut self, index: usize) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.index == index) {
            step.status = StepStatus::InProgress;
            self.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.save();
        }
    }

    /// Get the next pending step index, or None if all done.
    pub fn next_pending(&self) -> Option<usize> {
        self.steps
            .iter()
            .find(|s| s.status == StepStatus::Pending)
            .map(|s| s.index)
    }

    /// Summary string for display.
    pub fn summary(&self) -> String {
        let done = self.steps.iter().filter(|s| s.status == StepStatus::Done).count();
        let total = self.steps.len();
        format!(
            "Plan: {} ({}/{} steps done)",
            if self.task.len() > 50 {
                let end = {
                    let mut e = 50_usize.min(self.task.len());
                    while e > 0 && !self.task.is_char_boundary(e) { e -= 1; }
                    e
                };
                &self.task[..end]
            } else { &self.task },
            done,
            total
        )
    }

    /// Is the plan complete?
    pub fn is_complete(&self) -> bool {
        self.steps
            .iter()
            .all(|s| matches!(s.status, StepStatus::Done | StepStatus::Skipped))
    }
}
