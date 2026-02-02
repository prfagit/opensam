//! OPERATIVE TOOLKIT

pub mod filesystem;
pub mod message;
pub mod shell;
pub mod web;
// pub mod spawn;  // Disabled - subagent support not yet implemented
pub mod path_utils;

pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use message::MessageTool;
pub use shell::ExecTool;
pub use web::{WebFetchTool, WebSearchTool};
// pub use spawn::SpawnTool;  // Disabled - subagent support not yet implemented

use async_trait::async_trait;
use opensam_provider::Tool;
use serde_json::Value;
use std::collections::HashMap;

/// TOOLKIT trait
type BoxedTool = Box<dyn ToolTrait + Send + Sync>;

#[async_trait]
pub trait ToolTrait: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    async fn execute(
        &self,
        args: Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

pub fn to_provider_tool(tool: &dyn ToolTrait) -> Tool {
    Tool::new(tool.name(), tool.description(), tool.parameters())
}

/// TOOLKIT registry
pub struct ToolRegistry {
    tools: HashMap<String, BoxedTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: ToolTrait + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        self.tools.insert(name, Box::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<&(dyn ToolTrait + Send + Sync)> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub fn definitions(&self) -> Vec<Tool> {
        self.tools
            .values()
            .map(|t| to_provider_tool(t.as_ref()))
            .collect()
    }

    pub async fn execute(
        &self,
        name: &str,
        args: Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| format!("◆ TOOLKIT '{}' NOT FOUND", name))?;
        tool.execute(args).await
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Register default tools with the given workspace
pub fn register_default_tools(
    registry: &mut ToolRegistry,
    brave_key: Option<String>,
    workspace: &std::path::Path,
    _bus: opensam_bus::MessageBus,
) {
    // Filesystem tools
    registry.register(ReadFileTool::new(workspace.to_path_buf()));
    registry.register(WriteFileTool::new(workspace.to_path_buf()));
    registry.register(EditFileTool::new(workspace.to_path_buf()));
    registry.register(ListDirTool::new(workspace.to_path_buf()));

    // Shell tool
    registry.register(ExecTool::with_workspace(workspace.to_path_buf()));

    // Web tools
    registry.register(WebSearchTool::new(brave_key, 5));
    registry.register(WebFetchTool::default());

    // Message tool
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    registry.register(MessageTool::new(sender));
}
