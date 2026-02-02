//! TOOLKIT: Terminal Operations

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::debug;

use super::path_utils::validate_workspace_path;
use super::ToolTrait;

/// Terminal command tool
pub struct ExecTool {
    timeout_secs: u64,
    working_dir: Option<String>,
    workspace: PathBuf,
}

impl ExecTool {
    pub fn new(timeout_secs: u64, working_dir: Option<String>, workspace: PathBuf) -> Self {
        Self {
            timeout_secs,
            working_dir,
            workspace,
        }
    }
    pub fn with_workspace(workspace: PathBuf) -> Self {
        Self {
            timeout_secs: 60,
            working_dir: None,
            workspace,
        }
    }
}

#[derive(Deserialize)]
struct ExecArgs {
    command: String,
    working_dir: Option<String>,
}

#[async_trait]
impl ToolTrait for ExecTool {
    fn name(&self) -> &str {
        "exec"
    }
    fn description(&self) -> &str {
        "Execute terminal command. Use with caution."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Terminal command" },
                "working_dir": { "type": "string", "description": "Optional working directory" }
            },
            "required": ["command"]
        })
    }
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: ExecArgs = serde_json::from_value(args)?;

        // Determine working directory: args > tool config > workspace default
        let working_dir = match args.working_dir.or_else(|| self.working_dir.clone()) {
            Some(dir) => {
                // Validate the provided working directory is within workspace
                Some(validate_workspace_path(&dir, &self.workspace).await?)
            }
            None => {
                // Default to workspace root
                Some(self.workspace.clone())
            }
        };

        debug!("◆ EXECUTING: {}", args.command);
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(&args.command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        let result = match tokio::time::timeout(
            tokio::time::Duration::from_secs(self.timeout_secs),
            cmd.output(),
        )
        .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Ok(format!("◆ EXECUTION FAILED: {}", e)),
            Err(_) => return Ok(format!("◆ TIMEOUT AFTER {} SECONDS", self.timeout_secs)),
        };
        let mut parts = Vec::new();
        if !result.stdout.is_empty() {
            parts.push(String::from_utf8_lossy(&result.stdout).to_string());
        }
        if !result.stderr.is_empty() {
            parts.push(format!(
                "STDERR:\n{}",
                String::from_utf8_lossy(&result.stderr)
            ));
        }
        if result.status.code() != Some(0) {
            parts.push(format!("EXIT CODE: {}", result.status.code().unwrap_or(-1)));
        }
        let result = if parts.is_empty() {
            "(NO OUTPUT)".to_string()
        } else {
            parts.join("\n")
        };
        const MAX_LEN: usize = 10000;
        if result.len() > MAX_LEN {
            Ok(format!(
                "{}\n◆ OUTPUT TRUNCATED: {} BYTES REMAINING",
                &result[..MAX_LEN],
                result.len() - MAX_LEN
            ))
        } else {
            Ok(result)
        }
    }
}
