// TODO: Implement real subagent support
// This tool is currently disabled from the default registry.
// Future implementation should:
// - Add a SubagentManager that spawns tasks using tokio::spawn
// - Track task status and allow reporting
// - Handle subagent lifecycle (spawn, monitor, terminate)

//! Spawn tool for creating subagents

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::ToolTrait;

/// Tool for spawning background subagents
pub struct SpawnTool {
    context_channel: std::sync::Mutex<Option<String>>,
    context_chat_id: std::sync::Mutex<Option<String>>,
}

impl SpawnTool {
    /// Create a new spawn tool
    pub fn new() -> Self {
        Self {
            context_channel: std::sync::Mutex::new(None),
            context_chat_id: std::sync::Mutex::new(None),
        }
    }

    /// Set the context for spawned tasks
    pub fn set_context(&self, channel: String, chat_id: String) {
        *self.context_channel.lock().unwrap() = Some(channel);
        *self.context_chat_id.lock().unwrap() = Some(chat_id);
    }
}

#[derive(Deserialize)]
struct SpawnArgs {
    task: String,
    #[serde(default)]
    label: Option<String>,
}

#[async_trait]
impl ToolTrait for SpawnTool {
    fn name(&self) -> &str { "spawn" }
    fn description(&self) -> &str { "Spawn a subagent to execute a task in the background." }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "Task description for the subagent" },
                "label": { "type": "string", "description": "Optional label for the task" }
            },
            "required": ["task"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: SpawnArgs = serde_json::from_value(args)?;
        let label = args.label.unwrap_or_else(|| args.task.clone());
        
        Ok(format!("Subagent [{}] would be spawned (not yet implemented)", label))
    }
}

impl Default for SpawnTool {
    fn default() -> Self { Self::new() }
}
