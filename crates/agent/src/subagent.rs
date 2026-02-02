//! Subagent manager for background task execution

use std::collections::HashMap;
use tokio::task::JoinHandle;
use tracing::info;

use opensam_bus::MessageBus;
use opensam_provider::Provider;

/// Manages background subagents
#[allow(dead_code)]
pub struct SubagentManager<P: Provider> {
    provider: std::sync::Arc<P>,
    workspace: std::path::PathBuf,
    bus: MessageBus,
    model: String,
    brave_api_key: Option<String>,
    running_tasks: HashMap<String, JoinHandle<()>>,
}

impl<P: Provider> SubagentManager<P> {
    /// Create a new subagent manager
    pub fn new(
        provider: P,
        workspace: std::path::PathBuf,
        bus: MessageBus,
        model: String,
        brave_api_key: Option<String>,
    ) -> Self {
        Self {
            provider: std::sync::Arc::new(provider),
            workspace,
            bus,
            model,
            brave_api_key,
            running_tasks: HashMap::new(),
        }
    }

    /// Spawn a new subagent task
    pub fn spawn(&mut self, task: String, label: Option<String>) -> String {
        let id = format!("task_{}", self.running_tasks.len());
        let label = label.unwrap_or_else(|| task.clone());

        info!("Spawning subagent [{}]: {}", id, label);

        // Simplified - just log for now
        // In full implementation, would spawn actual async task

        id
    }

    /// Get count of running subagents
    pub fn running_count(&self) -> usize {
        self.running_tasks.len()
    }
}
