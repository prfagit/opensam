//! Tests for path utilities

use opensam_config::paths::{ensure_dir, safe_filename};

use tempfile::TempDir;

/// Helper to create a temporary directory
fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Test safe_filename with special characters
#[test]
fn test_safe_filename_special_chars() {
    assert_eq!(safe_filename("file<name>"), "file_name_");
    assert_eq!(safe_filename("file:name"), "file_name");
    assert_eq!(safe_filename("file\"name"), "file_name");
    assert_eq!(safe_filename("file/name"), "file_name");
    assert_eq!(safe_filename("file\\name"), "file_name");
    assert_eq!(safe_filename("file|name"), "file_name");
    assert_eq!(safe_filename("file?name"), "file_name");
    assert_eq!(safe_filename("file*name"), "file_name");
}

/// Test safe_filename with multiple special chars
#[test]
fn test_safe_filename_multiple_special() {
    assert_eq!(safe_filename("<>:\"/\\|?*"), "_________");
    assert_eq!(safe_filename("file<>name:test"), "file__name_test");
}

/// Test safe_filename with normal characters (no change)
#[test]
fn test_safe_filename_normal() {
    assert_eq!(safe_filename("normal_filename"), "normal_filename");
    assert_eq!(safe_filename("file.txt"), "file.txt");
    assert_eq!(safe_filename("UPPER_CASE"), "UPPER_CASE");
    assert_eq!(safe_filename("mixed-Case_123"), "mixed-Case_123");
}

/// Test safe_filename with empty string
#[test]
fn test_safe_filename_empty() {
    assert_eq!(safe_filename(""), "");
}

/// Test safe_filename with spaces (should be preserved)
#[test]
fn test_safe_filename_spaces() {
    assert_eq!(
        safe_filename("file name with spaces"),
        "file name with spaces"
    );
}

/// Test safe_filename with unicode
#[test]
fn test_safe_filename_unicode() {
    assert_eq!(safe_filename("文件📄name"), "文件📄name");
    assert_eq!(safe_filename("日本語ファイル"), "日本語ファイル");
}

/// Test ensure_dir creates directory
#[tokio::test]
async fn test_ensure_dir_creates_directory() {
    let temp_dir = temp_dir();
    let new_dir = temp_dir.path().join("new_directory");

    assert!(!new_dir.exists());

    ensure_dir(&new_dir).await.expect("Failed to ensure dir");

    assert!(new_dir.exists());
    assert!(new_dir.is_dir());
}

/// Test ensure_dir creates nested directories
#[tokio::test]
async fn test_ensure_dir_nested() {
    let temp_dir = temp_dir();
    let nested = temp_dir.path().join("a/b/c/d");

    ensure_dir(&nested)
        .await
        .expect("Failed to ensure nested dir");

    assert!(nested.exists());
    assert!(nested.is_dir());
}

/// Test ensure_dir is idempotent
#[tokio::test]
async fn test_ensure_dir_idempotent() {
    let temp_dir = temp_dir();
    let dir = temp_dir.path().join("existing");

    // Create first time
    ensure_dir(&dir).await.expect("Failed first create");
    assert!(dir.exists());

    // Create again (should not fail)
    ensure_dir(&dir).await.expect("Failed second create");
    assert!(dir.exists());
}

/// Test ensure_dir on already existing file (should fail)
#[tokio::test]
async fn test_ensure_dir_on_file() {
    let temp_dir = temp_dir();
    let file_path = temp_dir.path().join("a_file");

    // Create a file
    tokio::fs::write(&file_path, "content")
        .await
        .expect("Failed to write file");
    assert!(file_path.exists());

    // Try to create dir with same path (should fail)
    let result = ensure_dir(&file_path).await;
    assert!(result.is_err());
}

/// Test data_dir returns expected path
#[test]
fn test_data_dir() {
    use opensam_config::paths::data_dir;

    let dir = data_dir();
    let home = dirs::home_dir().expect("No home dir");

    assert_eq!(dir, home.join(".opensam"));
}

/// Test config_path returns expected path
#[test]
fn test_config_path() {
    use opensam_config::paths::config_path;

    let path = config_path();
    let home = dirs::home_dir().expect("No home dir");

    assert_eq!(path, home.join(".opensam/config.json"));
}

/// Test workspace_path returns expected path
#[test]
fn test_workspace_path() {
    use opensam_config::paths::workspace_path;

    let path = workspace_path();
    let home = dirs::home_dir().expect("No home dir");

    assert_eq!(path, home.join(".opensam/ops"));
}

/// Test sessions_dir returns expected path
#[test]
fn test_sessions_dir() {
    use opensam_config::paths::sessions_dir;

    let path = sessions_dir();
    let home = dirs::home_dir().expect("No home dir");

    assert_eq!(path, home.join(".opensam/logs"));
}

/// Test cron_dir returns expected path
#[test]
fn test_cron_dir() {
    use opensam_config::paths::cron_dir;

    let path = cron_dir();
    let home = dirs::home_dir().expect("No home dir");

    assert_eq!(path, home.join(".opensam/timeline"));
}

/// Test media_dir returns expected path
#[test]
fn test_media_dir() {
    use opensam_config::paths::media_dir;

    let path = media_dir();
    let home = dirs::home_dir().expect("No home dir");

    assert_eq!(path, home.join(".opensam/intel"));
}

/// Test all path functions return absolute paths
#[test]
fn test_all_paths_absolute() {
    use opensam_config::paths::*;

    assert!(data_dir().is_absolute());
    assert!(config_path().is_absolute());
    assert!(workspace_path().is_absolute());
    assert!(sessions_dir().is_absolute());
    assert!(cron_dir().is_absolute());
    assert!(media_dir().is_absolute());
}

/// Test that all dirs are under .opensam
#[test]
fn test_all_dirs_under_opensam() {
    use opensam_config::paths::*;

    let data = data_dir();

    assert!(config_path().starts_with(&data));
    assert!(workspace_path().starts_with(&data));
    assert!(sessions_dir().starts_with(&data));
    assert!(cron_dir().starts_with(&data));
    assert!(media_dir().starts_with(&data));
}
