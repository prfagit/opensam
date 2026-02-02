//! Context builder for assembling agent prompts

use chrono::Local;
use std::path::{Path, PathBuf};
use tracing::debug;

use opensam_provider::Message;

/// Builds context (system prompt + messages) for the agent
pub struct ContextBuilder {
    workspace: PathBuf,
}

impl ContextBuilder {
    /// Bootstrap files to load
    const BOOTSTRAP_FILES: &[&str] = &["DIRECTIVE.md", "PERSONA.md", "SUBJECT.md"];

    /// Create a new context builder
    pub fn new(workspace: impl AsRef<Path>) -> Self {
        Self {
            workspace: workspace.as_ref().to_path_buf(),
        }
    }

    /// Build the system prompt
    pub async fn build_system_prompt(&self) -> String {
        let mut parts = vec![self.identity()];

        // Load bootstrap files
        if let Ok(bootstrap) = self.load_bootstrap_files().await {
            if !bootstrap.is_empty() {
                parts.push(bootstrap);
            }
        }

        // Memory context
        if let Ok(memory) = self.load_memory().await {
            if !memory.is_empty() {
                parts.push(format!("# Memory\n\n{}", memory));
            }
        }

        parts.join("\n\n---\n\n")
    }

    fn identity(&self) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M (%A)");
        let workspace_path = self.workspace.display();

        format!(
            r#"# opensam

You are opensam, a helpful AI assistant. You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Search the web and fetch web pages
- Send messages to users on chat channels
- Spawn subagents for complex background tasks

## Current Time
{}

## Workspace
Your workspace is at: {}
- Memory files: {}/lifepod/MEMORY.md

IMPORTANT: When responding to direct questions or conversations, reply directly with your text response.
Only use the 'message' tool when you need to send a message to a specific chat channel (like WhatsApp).
For normal conversation, just respond with text - do not call the message tool.

Always be helpful, accurate, and concise. When using tools, explain what you're doing.
When remembering something, write to {}/lifepod/MEMORY.md"#,
            now, workspace_path, workspace_path, workspace_path
        )
    }

    async fn load_bootstrap_files(&self) -> std::io::Result<String> {
        let mut parts = Vec::new();

        for filename in Self::BOOTSTRAP_FILES {
            let path = self.workspace.join(filename);
            if path.exists() {
                match tokio::fs::read_to_string(&path).await {
                    Ok(content) => {
                        parts.push(format!("## {}\n\n{}", filename, content));
                    }
                    Err(e) => debug!("Failed to read {}: {}", filename, e),
                }
            }
        }

        Ok(parts.join("\n\n"))
    }

    async fn load_memory(&self) -> std::io::Result<String> {
        let memory_path = self.workspace.join("lifepod").join("MEMORY.md");
        if memory_path.exists() {
            tokio::fs::read_to_string(&memory_path).await
        } else {
            Ok(String::new())
        }
    }

    /// Build complete messages list for LLM
    pub async fn build_messages(
        &self,
        history: Vec<Message>,
        current_message: &str,
    ) -> Vec<Message> {
        let system_prompt = self.build_system_prompt().await;

        let mut messages = vec![Message::system(system_prompt)];
        messages.extend(history);
        messages.push(Message::user(current_message));

        messages
    }

    /// Add a tool result to messages
    pub fn add_tool_result(
        messages: &mut Vec<Message>,
        tool_call_id: &str,
        name: &str,
        result: &str,
    ) {
        messages.push(Message::tool(tool_call_id, name, result));
    }

    /// Add an assistant message with tool calls
    pub fn add_assistant_message(
        messages: &mut Vec<Message>,
        content: Option<&str>,
        tool_calls: Option<Vec<opensam_provider::ToolCallDef>>,
    ) {
        let mut msg = Message::assistant(content.unwrap_or(""));
        if let Some(calls) = tool_calls {
            msg.tool_calls = Some(calls);
        }
        messages.push(msg);
    }
}
