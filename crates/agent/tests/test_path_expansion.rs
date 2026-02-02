//! Tests for path expansion functionality (within workspace context)

use opensam_agent::tools::filesystem::ReadFileTool;
use opensam_agent::tools::ToolTrait;
use opensam_config::workspace_path;
use serde_json::json;
use std::env;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_expand_path_with_tilde_in_workspace() {
    // Create a test file in the workspace using tilde expansion
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_file = workspace.join(".opensam_test_expand.txt");
    fs::write(&test_file, "tilde test content").unwrap();

    // Use tilde path pointing to workspace
    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": "~/.opensam/ops/.opensam_test_expand.txt"});

    let result = tool.execute(args).await.unwrap();
    assert_eq!(result, "tilde test content");

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_expand_path_absolute_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_file = workspace.join("absolute_test.txt");
    fs::write(&test_file, "absolute path content").unwrap();

    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": test_file.to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert_eq!(result, "absolute path content");

    // Cleanup
    let _ = fs::remove_file(&test_file);
}

#[tokio::test]
async fn test_expand_path_relative_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    // Create a subdirectory in workspace
    let subdir = workspace.join("test_relative");
    let _ = fs::create_dir_all(&subdir);

    let test_file = subdir.join("relative.txt");
    fs::write(&test_file, "relative path content").unwrap();

    // Use relative path from workspace root (e.g., "test_relative/relative.txt")
    // The tool resolves relative paths against the workspace, not current dir
    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": "test_relative/relative.txt"});

    let result = tool.execute(args).await;

    // Should succeed - path is within workspace
    assert!(
        result.is_ok(),
        "Should read file with relative path: {:?}",
        result
    );
    assert_eq!(result.unwrap(), "relative path content");

    // Cleanup
    let _ = fs::remove_dir_all(&subdir);
}

#[tokio::test]
async fn test_expand_path_outside_workspace_rejected() {
    // Create temp dir outside of workspace
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("outside.txt");
    fs::write(&test_file, "outside content").unwrap();

    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": test_file.to_str().unwrap()});

    let result = tool.execute(args).await;

    // Should fail with workspace validation error
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("is outside workspace"));
}

#[tokio::test]
async fn test_expand_path_with_home_in_workspace() {
    let _home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap();
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    // Test file in workspace, accessed via absolute home path
    let test_file = workspace.join("home_path_test.txt");
    fs::write(&test_file, "home path content").unwrap();

    let tool = ReadFileTool::new(workspace_path());
    let args = json!({"path": test_file.to_str().unwrap()});

    let result = tool.execute(args).await.unwrap();
    assert_eq!(result, "home path content");

    // Cleanup
    let _ = fs::remove_file(&test_file);
}
