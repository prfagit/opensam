//! Tests for tool registry

use opensam_agent::tools::{
    to_provider_tool, EditFileTool, ExecTool, ListDirTool, ReadFileTool, ToolRegistry, ToolTrait,
    WebFetchTool, WebSearchTool, WriteFileTool,
};
use serde_json::json;

#[test]
fn test_registry_new() {
    let registry = ToolRegistry::new();
    assert!(registry.names().is_empty());
}

#[test]
fn test_registry_default() {
    let registry: ToolRegistry = Default::default();
    assert!(registry.names().is_empty());
}

#[test]
fn test_registry_register_single() {
    let mut registry = ToolRegistry::new();
    registry.register(ReadFileTool::new(std::path::PathBuf::from("/tmp")));

    assert_eq!(registry.names().len(), 1);
    assert!(registry.has("read_file"));
    assert!(registry.names().contains(&"read_file".to_string()));
}

#[test]
fn test_registry_register_multiple() {
    let mut registry = ToolRegistry::new();
    registry.register(ReadFileTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(WriteFileTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(EditFileTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(ListDirTool::new(std::path::PathBuf::from("/tmp")));

    assert_eq!(registry.names().len(), 4);
    assert!(registry.has("read_file"));
    assert!(registry.has("write_file"));
    assert!(registry.has("edit_file"));
    assert!(registry.has("list_dir"));
}

#[test]
fn test_registry_get_existing() {
    let mut registry = ToolRegistry::new();
    registry.register(ReadFileTool::new(std::path::PathBuf::from("/tmp")));

    let tool = registry.get("read_file");
    assert!(tool.is_some());
    assert_eq!(tool.unwrap().name(), "read_file");
}

#[test]
fn test_registry_get_missing() {
    let registry = ToolRegistry::new();

    let tool = registry.get("nonexistent");
    assert!(tool.is_none());
}

#[test]
fn test_registry_has() {
    let mut registry = ToolRegistry::new();
    registry.register(ExecTool::with_workspace(std::path::PathBuf::from("/tmp")));

    assert!(registry.has("exec"));
    assert!(!registry.has("nonexistent"));
}

#[test]
fn test_registry_definitions() {
    let mut registry = ToolRegistry::new();
    registry.register(ReadFileTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(WriteFileTool::new(std::path::PathBuf::from("/tmp")));

    let definitions = registry.definitions();
    assert_eq!(definitions.len(), 2);

    // Check that definitions are properly converted
    let names: Vec<String> = definitions
        .iter()
        .map(|d| d.function.name.clone())
        .collect();
    assert!(names.contains(&"read_file".to_string()));
    assert!(names.contains(&"write_file".to_string()));
}

#[tokio::test]
async fn test_registry_execute_not_found() {
    let registry = ToolRegistry::new();

    let args = json!({"test": "value"});
    let result = registry.execute("nonexistent", args).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("NOT FOUND") || err.contains("not found"));
}

#[test]
fn test_registry_names() {
    let mut registry = ToolRegistry::new();

    assert!(registry.names().is_empty());

    registry.register(WebSearchTool::new(None, 5));
    registry.register(WebFetchTool::default());

    let names = registry.names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"web_search".to_string()));
    assert!(names.contains(&"web_fetch".to_string()));
}

#[test]
fn test_to_provider_tool() {
    let tool = ReadFileTool::new(std::path::PathBuf::from("/tmp"));
    let provider_tool = to_provider_tool(&tool);

    assert_eq!(provider_tool.function.name, "read_file");
    assert_eq!(
        provider_tool.function.description,
        "Retrieve intel from data store at specified path."
    );
}

#[test]
fn test_tool_trait_methods() {
    let tool = ReadFileTool::new(std::path::PathBuf::from("/tmp"));

    assert_eq!(tool.name(), "read_file");
    assert_eq!(
        tool.description(),
        "Retrieve intel from data store at specified path."
    );

    let params = tool.parameters();
    assert_eq!(params["type"], "object");
}

#[tokio::test]
async fn test_full_toolkit_registry() {
    let mut registry = ToolRegistry::new();

    // Register all tools
    registry.register(ReadFileTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(WriteFileTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(EditFileTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(ListDirTool::new(std::path::PathBuf::from("/tmp")));
    registry.register(ExecTool::with_workspace(std::path::PathBuf::from("/tmp")));
    registry.register(WebSearchTool::new(None, 5));
    registry.register(WebFetchTool::default());

    // Verify all tools are registered
    assert_eq!(registry.names().len(), 7);
    assert!(registry.has("read_file"));
    assert!(registry.has("write_file"));
    assert!(registry.has("edit_file"));
    assert!(registry.has("list_dir"));
    assert!(registry.has("exec"));
    assert!(registry.has("web_search"));
    assert!(registry.has("web_fetch"));

    // Verify definitions
    let definitions = registry.definitions();
    assert_eq!(definitions.len(), 7);
}
