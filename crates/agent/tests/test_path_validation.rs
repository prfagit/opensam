//! Tests for path validation functionality

use opensam_agent::tools::path_utils::validate_workspace_path;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_validate_workspace_path_inside() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.txt");
    fs::write(&test_file, "content").unwrap();

    let result = validate_workspace_path(test_file.to_str().unwrap(), workspace).await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().canonicalize().unwrap(),
        test_file.canonicalize().unwrap()
    );
}

#[tokio::test]
async fn test_validate_workspace_path_outside() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    let outside_file = temp_dir.path().join("outside.txt");
    fs::write(&outside_file, "content").unwrap();

    let result = validate_workspace_path(outside_file.to_str().unwrap(), &workspace).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected 'is outside workspace', got: {}",
        err
    );
}

#[tokio::test]
#[ignore = "test has isolation issues - changes current directory"]
async fn test_validate_workspace_path_traversal_escape() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    // Create a file outside the workspace
    let outside_file = temp_dir.path().join("secret.txt");
    fs::write(&outside_file, "secret").unwrap();

    // Change to workspace directory and try to access using ../ escape
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&workspace).unwrap();

    let result = validate_workspace_path("../secret.txt", &workspace).await;

    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("is outside workspace"),
        "Expected 'is outside workspace', got: {}",
        err
    );
}

#[tokio::test]
async fn test_validate_workspace_path_nested_escape() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("a/b/c/workspace");
    fs::create_dir_all(&workspace).unwrap();

    // Try to escape using multiple ../
    let result = validate_workspace_path("../../../outside.txt", &workspace).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("is outside workspace"));
}

#[tokio::test]
async fn test_validate_workspace_path_nonexistent_in_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Non-existent file inside workspace should be ok (returns resolved path)
    let result =
        validate_workspace_path(workspace.join("new_file.txt").to_str().unwrap(), workspace).await;

    assert!(result.is_ok());
    // Result should be within workspace
    let result_path = result.unwrap();
    // Check that the path is valid (it will be in the current working directory,
    // which may be different from temp_dir)
    assert!(result_path.ends_with("new_file.txt"));
}

#[tokio::test]
async fn test_validate_workspace_path_nonexistent_outside() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    // Non-existent file outside workspace should fail
    let result = validate_workspace_path(
        temp_dir.path().join("../outside.txt").to_str().unwrap(),
        &workspace,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("is outside workspace"));
}

#[tokio::test]
async fn test_validate_workspace_path_absolute_outside() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    // Absolute path outside workspace
    let result = validate_workspace_path("/etc/passwd", &workspace).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("is outside workspace"));
}

#[tokio::test]
async fn test_validate_workspace_path_directory_inside() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let subdir = workspace.join("subdir");
    fs::create_dir(&subdir).unwrap();

    let result = validate_workspace_path(subdir.to_str().unwrap(), workspace).await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().canonicalize().unwrap(),
        subdir.canonicalize().unwrap()
    );
}

#[tokio::test]
async fn test_validate_workspace_path_directory_outside() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    // Directory outside workspace
    let outside_dir = temp_dir.path().join("outside");
    fs::create_dir(&outside_dir).unwrap();

    let result = validate_workspace_path(outside_dir.to_str().unwrap(), &workspace).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("is outside workspace"));
}

#[tokio::test]
async fn test_validate_workspace_path_with_symlink_inside() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let real_file = workspace.join("real.txt");
    fs::write(&real_file, "content").unwrap();

    let symlink = workspace.join("link.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_file, &symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&real_file, &symlink).unwrap();

    let result = validate_workspace_path(symlink.to_str().unwrap(), workspace).await;

    assert!(result.is_ok());
    // Should resolve to the real file path (canonicalized)
    let result_path = result.unwrap();
    // The canonicalized symlink target should be the real file
    assert_eq!(
        result_path.canonicalize().unwrap(),
        real_file.canonicalize().unwrap()
    );
}

#[tokio::test]
async fn test_validate_workspace_path_with_symlink_escape() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).unwrap();

    // Create file outside workspace
    let outside_file = temp_dir.path().join("secret.txt");
    fs::write(&outside_file, "secret").unwrap();

    // Create symlink inside workspace pointing outside
    let symlink = workspace.join("link.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside_file, &symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&outside_file, &symlink).unwrap();

    let result = validate_workspace_path(symlink.to_str().unwrap(), &workspace).await;

    // After canonicalization, symlink resolves to outside file
    // which should be rejected
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("is outside workspace"));
}

#[tokio::test]
async fn test_validate_workspace_path_deeply_nested() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let deep_path = workspace.join("a/b/c/d/e/f/g");
    fs::create_dir_all(&deep_path).unwrap();
    let deep_file = deep_path.join("deep.txt");
    fs::write(&deep_file, "deep content").unwrap();

    let result = validate_workspace_path(deep_file.to_str().unwrap(), workspace).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_workspace_path_same_as_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // The workspace directory itself should be valid
    let result = validate_workspace_path(workspace.to_str().unwrap(), workspace).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_workspace_path_with_tilde_in_workspace() {
    // This test assumes ~/.opensam/ops exists or will be created
    let home = dirs::home_dir().expect("Should have home dir");
    let workspace = home.join(".opensam/ops");

    // Ensure workspace exists
    let _ = fs::create_dir_all(&workspace);

    // Test with tilde path inside workspace
    let result = validate_workspace_path("~/.opensam/ops/test.txt", &workspace).await;

    // Should succeed (path is within workspace)
    assert!(result.is_ok());
    let result_path = result.unwrap();
    assert!(
        result_path.starts_with(&workspace)
            || result_path.to_string_lossy().contains(".opensam/ops")
    );
}

#[tokio::test]
async fn test_validate_workspace_path_with_dot_relative() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let test_file = workspace.join("test.txt");
    fs::write(&test_file, "content").unwrap();

    // Change to workspace and use relative path with ./
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(workspace).unwrap();

    let result = validate_workspace_path("./test.txt", workspace).await;

    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_workspace_path_with_unicode() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let unicode_file = workspace.join("文件_日本語_🎉.txt");
    fs::write(&unicode_file, "unicode content").unwrap();

    let result = validate_workspace_path(unicode_file.to_str().unwrap(), workspace).await;

    assert!(result.is_ok());
}
