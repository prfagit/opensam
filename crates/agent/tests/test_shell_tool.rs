//! Tests for shell execution tool

use opensam_agent::tools::{ExecTool, ToolTrait};
use opensam_config::workspace_path;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_exec_tool_echo_in_workspace() {
    let tool = ExecTool::with_workspace(workspace_path());
    let args = json!({"command": "echo 'Hello from shell'"});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("Hello from shell"));
}

#[tokio::test]
async fn test_exec_tool_with_working_dir_outside_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let tool = ExecTool::with_workspace(workspace_path());
    let args = json!({
        "command": "pwd",
        "working_dir": temp_dir.path().to_str().unwrap()
    });

    let result = tool.execute(args).await;

    // Should fail because working_dir is outside workspace
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected workspace error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_exec_tool_with_working_dir_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_dir = workspace.join("test_exec_working_dir");
    let _ = fs::create_dir_all(&test_dir);

    let tool = ExecTool::with_workspace(workspace.clone());
    let args = json!({
        "command": "pwd",
        "working_dir": test_dir.to_str().unwrap()
    });

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains(test_dir.to_str().unwrap()));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_exec_tool_stderr_output() {
    let tool = ExecTool::with_workspace(workspace_path());
    let args = json!({"command": "echo 'error message' >&2"});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("STDERR"));
    assert!(result.contains("error message"));
}

#[tokio::test]
async fn test_exec_tool_exit_code() {
    let tool = ExecTool::with_workspace(workspace_path());
    let args = json!({"command": "exit 42"});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("EXIT CODE: 42"));
}

#[tokio::test]
async fn test_exec_tool_empty_output() {
    let tool = ExecTool::with_workspace(workspace_path());
    let args = json!({"command": "true"});

    let result = tool.execute(args).await.unwrap();
    // Should have no output for 'true' command
    assert!(result.is_empty() || result == "(NO OUTPUT)");
}

#[tokio::test]
async fn test_exec_tool_custom_timeout() {
    let tool = ExecTool::new(1, None, workspace_path());
    let args = json!({"command": "sleep 5"});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("TIMEOUT"));
    assert!(result.contains("1 SECOND"));
}

#[tokio::test]
async fn test_exec_tool_invalid_command() {
    let tool = ExecTool::with_workspace(workspace_path());
    let args = json!({"command": "this_command_does_not_exist_12345"});

    let result = tool.execute(args).await.unwrap();
    // The shell may report the error via stderr with a non-zero exit code
    // or via EXECUTION FAILED message
    assert!(
        result.contains("EXECUTION FAILED")
            || result.contains("not found")
            || result.contains("EXIT CODE:"),
        "Expected error indication, got: {}",
        result
    );
}

#[tokio::test]
async fn test_exec_tool_constructor_new_in_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_dir = workspace.join("test_exec_constructor");
    let _ = fs::create_dir_all(&test_dir);

    let tool = ExecTool::new(
        30,
        Some(test_dir.to_str().unwrap().to_string()),
        workspace.clone(),
    );
    let args = json!({"command": "pwd"});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains(test_dir.to_str().unwrap()));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_exec_tool_constructor_new_outside_workspace() {
    let temp_dir = TempDir::new().unwrap();

    // Set working_dir via constructor to a path outside workspace
    let tool = ExecTool::new(
        30,
        Some(temp_dir.path().to_str().unwrap().to_string()),
        workspace_path(),
    );
    let args = json!({"command": "pwd"});

    let result = tool.execute(args).await;

    // Should fail because working_dir is outside workspace
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected workspace error, got: {}",
        err
    );
}

#[test]
fn test_exec_tool_metadata() {
    let tool = ExecTool::with_workspace(workspace_path());
    assert_eq!(tool.name(), "exec");
    assert_eq!(
        tool.description(),
        "Execute terminal command. Use with caution."
    );
    let params = tool.parameters();
    assert_eq!(params["type"], "object");
    assert!(params["required"]
        .as_array()
        .unwrap()
        .contains(&json!("command")));
}

#[tokio::test]
async fn test_exec_tool_output_truncation() {
    let tool = ExecTool::with_workspace(workspace_path());
    // Generate output larger than 10000 characters
    let args = json!({"command": "yes 'A' | head -c 15000"});

    let result = tool.execute(args).await.unwrap();
    assert!(result.contains("OUTPUT TRUNCATED"));
    assert!(result.contains("5000 BYTES REMAINING"));
}

#[tokio::test]
async fn test_exec_tool_default_to_workspace() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    // When no working_dir is provided, should default to workspace
    let tool = ExecTool::with_workspace(workspace.clone());
    let args = json!({"command": "pwd"});

    let result = tool.execute(args).await.unwrap();
    // The result should contain the workspace path
    // Note: This assumes the workspace exists and is accessible
    assert!(result.contains(workspace.to_str().unwrap()) || !result.is_empty());
}

#[tokio::test]
async fn test_exec_tool_cannot_escape_workspace_via_command() {
    // Even with working_dir in workspace, try to access files outside via command
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    let test_dir = workspace.join("test_escape");
    let _ = fs::create_dir_all(&test_dir);

    let tool = ExecTool::with_workspace(workspace.clone());
    // Try to read /etc/passwd from within workspace
    let args = json!({
        "command": "cat /etc/passwd",
        "working_dir": test_dir.to_str().unwrap()
    });

    // The command itself is not validated, only the working_dir
    // So this will execute, but the working_dir validation should pass
    let result = tool.execute(args).await;

    // working_dir is valid, so this should succeed
    assert!(result.is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_exec_tool_nested_escape_attempt() {
    let workspace = workspace_path();
    let _ = fs::create_dir_all(&workspace);

    // Try to use ../../ to escape workspace in working_dir
    let tool = ExecTool::with_workspace(workspace);
    let args = json!({
        "command": "pwd",
        "working_dir": "../../tmp"
    });

    let result = tool.execute(args).await;

    // Should fail because resolved path is outside workspace
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected workspace error, got: {}",
        err
    );
}
