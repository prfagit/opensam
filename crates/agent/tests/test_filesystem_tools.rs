//! Tests for filesystem tools

use opensam_agent::tools::{EditFileTool, ListDirTool, ReadFileTool, ToolTrait, WriteFileTool};
use opensam_config::workspace_path;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Helper to run tests within the actual workspace directory

#[tokio::test]
async fn test_read_file_tool_outside_workspace() {
    // Create a temp directory outside the workspace
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("outside.txt");
    fs::write(&test_file, "secret").unwrap();

    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": test_file.to_str().unwrap()});

    let result = tool.execute(args).await;

    // Should fail with path validation error
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected workspace error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_write_file_tool_outside_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("outside.txt");

    let tool = WriteFileTool::new(workspace_path());
    let args = json!({
        "path": test_file.to_str().unwrap(),
        "content": "test"
    });

    let result = tool.execute(args).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected workspace error, got: {}",
        err
    );

    // File should not be created
    assert!(!test_file.exists());
}

#[tokio::test]
async fn test_edit_file_tool_outside_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("outside.txt");
    fs::write(&test_file, "content").unwrap();

    let tool = EditFileTool::new(workspace_path());
    let args = json!({
        "path": test_file.to_str().unwrap(),
        "old_text": "content",
        "new_text": "modified"
    });

    let result = tool.execute(args).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected workspace error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_list_dir_tool_outside_workspace() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ListDirTool::new(workspace_path());
    let args = json!({"path": temp_dir.path().to_str().unwrap()});

    let result = tool.execute(args).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected workspace error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_path_traversal_escape_attempt() {
    let workspace = workspace_path();

    // Try to escape using ../
    let tool = ReadFileTool::new(workspace);
    let args = json!({"path": "../../../../etc/passwd"});

    let result = tool.execute(args).await;

    // This may succeed if the resolved path is still within workspace,
    // or fail with path outside workspace
    // Either way, /etc/passwd should NOT be accessible
    match result {
        Ok(output) => {
            // If it succeeded, it should be because the file doesn't exist within workspace
            assert!(output.contains("NO INTEL AT"));
        }
        Err(e) => {
            // If it failed, it should be due to path validation
            let err = e.to_string();
            assert!(err.contains("is outside workspace") || err.contains("NO INTEL AT"));
        }
    }
}

#[tokio::test]
async fn test_read_file_tool_in_workspace() {
    let workspace = workspace_path();

    // Create a test file in the workspace
    let test_file = workspace.join("test_read_file.txt");
    let _ = fs::create_dir_all(&workspace);
    fs::write(&test_file, "Hello, Workspace!").unwrap();

    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": test_file.to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert_eq!(result, "Hello, Workspace!");

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_write_file_tool_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_file = workspace.join("test_write_file.txt");

    // Clean up if exists
    let _ = fs::remove_file(&test_file);

    let tool = WriteFileTool::new(workspace_path());
    let args = json!({
        "path": test_file.to_str().unwrap(),
        "content": "Test content in workspace"
    });

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("INTEL STORED"));
    assert!(result.contains("25 BYTES"));

    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "Test content in workspace");

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_edit_file_tool_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_file = workspace.join("test_edit_file.txt");
    fs::write(&test_file, "Hello, World!").unwrap();

    let tool = EditFileTool::new(workspace_path());
    let args = json!({
        "path": test_file.to_str().unwrap(),
        "old_text": "World",
        "new_text": "Rust"
    });

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("INTEL MODIFIED"));

    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "Hello, Rust!");

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_list_dir_tool_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_dir = workspace.join("test_list_dir");
    let _ = fs::create_dir_all(&test_dir);
    fs::write(test_dir.join("file1.txt"), "content1").unwrap();
    fs::write(test_dir.join("file2.txt"), "content2").unwrap();

    let tool = ListDirTool::new(workspace_path());
    let args = json!({"path": test_dir.to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("[FILE] file1.txt"));
    assert!(result.contains("[FILE] file2.txt"));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_read_file_tool_not_found_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": workspace.join("nonexistent_file_xyz.txt").to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("NO INTEL AT"));
}

#[tokio::test]
async fn test_read_file_tool_not_a_file_in_workspace() {
    let workspace = workspace_path();
    let test_dir = workspace.join("test_not_a_file_dir");
    let _ = fs::create_dir_all(&test_dir);

    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": test_dir.to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("NOT A DATA FILE"));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_write_file_tool_creates_directories_in_workspace() {
    let workspace = workspace_path();
    let nested_dir = workspace.join("test_nested/a/b/c");
    let nested_file = nested_dir.join("nested.txt");

    let tool = WriteFileTool::new(workspace_path());
    let args = json!({
        "path": nested_file.to_str().unwrap(),
        "content": "Nested content in workspace"
    });

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("INTEL STORED"));

    assert!(nested_file.exists());
    let content = fs::read_to_string(&nested_file).unwrap();
    assert_eq!(content, "Nested content in workspace");

    // Cleanup
    let _ = fs::remove_dir_all(workspace.join("test_nested"));
}

#[tokio::test]
async fn test_edit_file_tool_not_found_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let tool = EditFileTool::new(workspace_path());
    let args = json!({
        "path": workspace.join("nonexistent_edit.txt").to_str().unwrap(),
        "old_text": "old",
        "new_text": "new"
    });

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("NO INTEL AT"));
}

#[tokio::test]
async fn test_edit_file_tool_target_not_found_in_workspace() {
    let workspace = workspace_path();
    let test_file = workspace.join("test_edit_target.txt");
    let _ = fs::create_dir_all(&workspace);
    fs::write(&test_file, "Some content").unwrap();

    let tool = EditFileTool::new(workspace_path());
    let args = json!({
        "path": test_file.to_str().unwrap(),
        "old_text": "nonexistent",
        "new_text": "replacement"
    });

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("TARGET SEGMENT NOT FOUND"));

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_edit_file_tool_ambiguous_target_in_workspace() {
    let workspace = workspace_path();
    let test_file = workspace.join("test_edit_ambiguous.txt");
    let _ = fs::create_dir_all(&workspace);
    fs::write(&test_file, "repeat repeat repeat").unwrap();

    let tool = EditFileTool::new(workspace_path());
    let args = json!({
        "path": test_file.to_str().unwrap(),
        "old_text": "repeat",
        "new_text": "once"
    });

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("AMBIGUOUS TARGET"));
    assert!(result.contains("3 MATCHES"));

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_list_dir_tool_not_found_in_workspace() {
    let workspace = workspace_path();

    let tool = ListDirTool::new(workspace_path());
    let args = json!({"path": workspace.join("nonexistent_dir_xyz").to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("NO DATA AT"));
}

#[tokio::test]
async fn test_list_dir_tool_not_a_directory_in_workspace() {
    let workspace = workspace_path();
    let test_file = workspace.join("test_not_a_dir.txt");
    let _ = fs::create_dir_all(&workspace);
    fs::write(&test_file, "content").unwrap();

    let tool = ListDirTool::new(workspace_path());
    let args = json!({"path": test_file.to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("NOT A DIRECTORY"));

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_list_dir_tool_empty_directory_in_workspace() {
    let workspace = workspace_path();
    let test_dir = workspace.join("test_empty_dir");
    let _ = fs::create_dir_all(&test_dir);

    let tool = ListDirTool::new(workspace_path());
    let args = json!({"path": test_dir.to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("EMPTY SECTOR"));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_filesystem_tool_metadata() {
    let read_tool = ReadFileTool::new(workspace_path());
    assert_eq!(read_tool.name(), "read_file");
    assert_eq!(
        read_tool.description(),
        "Retrieve intel from data store at specified path."
    );
    let params = read_tool.parameters();
    assert_eq!(params["type"], "object");
    assert!(params["required"]
        .as_array()
        .unwrap()
        .contains(&json!("path")));

    let write_tool = WriteFileTool::new(workspace_path());
    assert_eq!(write_tool.name(), "write_file");
    assert_eq!(
        write_tool.description(),
        "Store intel to data store. Creates secure directories if needed."
    );

    let edit_tool = EditFileTool::new(workspace_path());
    assert_eq!(edit_tool.name(), "edit_file");
    assert_eq!(
        edit_tool.description(),
        "Modify intel by replacing old_text with new_text. Must match exactly."
    );

    let list_tool = ListDirTool::new(workspace_path());
    assert_eq!(list_tool.name(), "list_dir");
    assert_eq!(
        list_tool.description(),
        "Reconnaissance: List contents of data directory."
    );
}
