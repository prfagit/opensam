//! TOOLKIT: File System Operations

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

use tracing::debug;

use super::path_utils::validate_workspace_path;
use super::ToolTrait;

/// INTEL retrieval tool
pub struct ReadFileTool {
    workspace: PathBuf,
}

impl ReadFileTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[derive(Deserialize)]
struct ReadFileArgs {
    path: String,
}

#[async_trait]
impl ToolTrait for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Retrieve intel from data store at specified path."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Target data path" } },
            "required": ["path"]
        })
    }
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: ReadFileArgs = serde_json::from_value(args)?;
        let path = validate_workspace_path(&args.path, &self.workspace).await?;

        debug!("◆ RETRIEVING INTEL: {:?}", path);
        if !path.exists() {
            return Ok(format!("◆ NO INTEL AT: {}", args.path));
        }
        if !path.is_file() {
            return Ok(format!("◆ NOT A DATA FILE: {}", args.path));
        }
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Ok(content),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Ok(format!("◆ ACCESS DENIED: {}", args.path))
            }
            Err(e) => Ok(format!("◆ RETRIEVAL ERROR: {}", e)),
        }
    }
}

/// Data insertion tool
pub struct WriteFileTool {
    workspace: PathBuf,
}

impl WriteFileTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[derive(Deserialize)]
struct WriteFileArgs {
    path: String,
    content: String,
}

#[async_trait]
impl ToolTrait for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Store intel to data store. Creates secure directories if needed."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Target storage path" },
                "content": { "type": "string", "description": "Intel to store" }
            },
            "required": ["path", "content"]
        })
    }
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: WriteFileArgs = serde_json::from_value(args)?;
        let path = validate_workspace_path(&args.path, &self.workspace).await?;

        debug!("◆ STORING INTEL: {:?}", path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        match tokio::fs::write(&path, &args.content).await {
            Ok(_) => Ok(format!(
                "◆ INTEL STORED: {} BYTES WRITTEN TO {}",
                args.content.len(),
                args.path
            )),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Ok(format!("◆ ACCESS DENIED: {}", args.path))
            }
            Err(e) => Ok(format!("◆ STORAGE ERROR: {}", e)),
        }
    }
}

/// Data modification tool
pub struct EditFileTool {
    workspace: PathBuf,
}

impl EditFileTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[derive(Deserialize)]
struct EditFileArgs {
    path: String,
    old_text: String,
    new_text: String,
}

#[async_trait]
impl ToolTrait for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }
    fn description(&self) -> &str {
        "Modify intel by replacing old_text with new_text. Must match exactly."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Target data path" },
                "old_text": { "type": "string", "description": "Intel segment to replace" },
                "new_text": { "type": "string", "description": "Replacement intel" }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: EditFileArgs = serde_json::from_value(args)?;
        let path = validate_workspace_path(&args.path, &self.workspace).await?;

        debug!("◆ MODIFYING INTEL: {:?}", path);
        if !path.exists() {
            return Ok(format!("◆ NO INTEL AT: {}", args.path));
        }
        let content = tokio::fs::read_to_string(&path).await?;
        if !content.contains(&args.old_text) {
            return Ok("◆ TARGET SEGMENT NOT FOUND".to_string());
        }
        let count = content.matches(&args.old_text).count();
        if count > 1 {
            return Ok(format!("◆ AMBIGUOUS TARGET: {} MATCHES", count));
        }
        let new_content = content.replacen(&args.old_text, &args.new_text, 1);
        match tokio::fs::write(&path, new_content).await {
            Ok(_) => Ok(format!("◆ INTEL MODIFIED: {}", args.path)),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Ok(format!("◆ ACCESS DENIED: {}", args.path))
            }
            Err(e) => Ok(format!("◆ MODIFICATION ERROR: {}", e)),
        }
    }
}

/// Directory reconnaissance tool
pub struct ListDirTool {
    workspace: PathBuf,
}

impl ListDirTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[derive(Deserialize)]
struct ListDirArgs {
    path: String,
}

#[async_trait]
impl ToolTrait for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }
    fn description(&self) -> &str {
        "Reconnaissance: List contents of data directory."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Target directory" } },
            "required": ["path"]
        })
    }
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: ListDirArgs = serde_json::from_value(args)?;
        let path = validate_workspace_path(&args.path, &self.workspace).await?;

        debug!("◆ RECON: {:?}", path);
        if !path.exists() {
            return Ok(format!("◆ NO DATA AT: {}", args.path));
        }
        if !path.is_dir() {
            return Ok(format!("◆ NOT A DIRECTORY: {}", args.path));
        }
        let mut entries = tokio::fs::read_dir(&path).await?;
        let mut items = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let prefix = if entry.file_type().await?.is_dir() {
                "[DIR] "
            } else {
                "[FILE] "
            };
            items.push(format!("{}{}", prefix, name));
        }
        items.sort();
        if items.is_empty() {
            Ok(format!("◆ EMPTY SECTOR: {}", args.path))
        } else {
            Ok(items.join("\n"))
        }
    }
}
