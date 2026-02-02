//! Tests for error handling

use opensam_config::ConfigError;
use std::io;
use std::path::PathBuf;

/// Test ConfigError::Io displays correctly
#[test]
fn test_io_error_display() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err = ConfigError::Io(io_err);

    let display = format!("{}", err);
    assert!(display.contains("DATA LINK ERROR"));
    assert!(display.contains("file not found"));
}

/// Test ConfigError::Json displays correctly
#[test]
fn test_json_error_display() {
    // Create a JSON error by parsing invalid JSON
    let json_err: serde_json::Error =
        serde_json::from_str::<serde_json::Value>("{invalid").unwrap_err();
    let err = ConfigError::Json(json_err);

    let display = format!("{}", err);
    assert!(display.contains("DECRYPTION FAILED"));
}

/// Test ConfigError::NotFound displays correctly
#[test]
fn test_not_found_error_display() {
    let path = PathBuf::from("/some/path");
    let err = ConfigError::NotFound(path.clone());

    let display = format!("{}", err);
    assert!(display.contains("INTEL NOT FOUND"));
    assert!(display.contains("/some/path"));
}

/// Test ConfigError implements std::error::Error
#[test]
fn test_error_trait() {
    fn check_error_trait<T: std::error::Error>() {}
    check_error_trait::<ConfigError>();
}

/// Test ConfigError::Io from io::Error conversion
#[test]
fn test_io_error_from() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "no permission");
    let err: ConfigError = io_err.into();

    match err {
        ConfigError::Io(_) => (), // Expected
        _ => panic!("Expected Io variant"),
    }
}

/// Test ConfigError::Json from serde_json::Error conversion
#[test]
fn test_json_error_from() {
    let result: Result<serde_json::Value, _> = serde_json::from_str("{ invalid json");
    let json_err = result.unwrap_err();
    let err: ConfigError = json_err.into();

    match err {
        ConfigError::Json(_) => (), // Expected
        _ => panic!("Expected Json variant"),
    }
}

/// Test ConfigError implements Debug
#[test]
fn test_error_debug() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
    let err = ConfigError::Io(io_err);

    let debug = format!("{:?}", err);
    assert!(debug.contains("Io"));
}

/// Test ConfigError Send + Sync
#[test]
fn test_error_send_sync() {
    fn check_send<T: Send>() {}
    fn check_sync<T: Sync>() {}

    check_send::<ConfigError>();
    check_sync::<ConfigError>();
}

/// Test loading from malformed JSON file (async)
#[tokio::test]
async fn test_load_from_malformed_json() {
    use opensam_config::Config;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("malformed.json");

    // Write invalid JSON
    tokio::fs::write(&config_path, "{ not valid json")
        .await
        .expect("Failed to write");

    // Try to load - should fail with Json error
    let result = Config::load_from(&config_path).await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Check it's a Json error
    match err {
        ConfigError::Json(_) => (), // Expected
        _ => panic!("Expected Json error, got {:?}", err),
    }
}

/// Test loading from directory (not a file)
#[tokio::test]
async fn test_load_from_directory() {
    use opensam_config::Config;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Try to load from directory path
    let result = Config::load_from(temp_dir.path()).await;

    // This should fail with Io error (is a directory)
    assert!(result.is_err());
}

/// Test loading from non-existent path returns default (not error)
#[tokio::test]
async fn test_load_nonexistent_returns_default_not_error() {
    use opensam_config::Config;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent = temp_dir.path().join("does_not_exist.json");

    // Should succeed with default config
    let result = Config::load_from(&nonexistent).await;
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.operative.defaults.model, "anthropic/claude-sonnet-4");
}

/// Test save to read-only directory fails with Io error
#[tokio::test]
async fn test_save_to_readonly_dir() {
    use opensam_config::Config;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("readonly/config.json");

    // Create readonly parent directory
    let parent = config_path.parent().unwrap();
    tokio::fs::create_dir_all(parent)
        .await
        .expect("Failed to create dir");

    let mut perms = tokio::fs::metadata(parent)
        .await
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o444); // Read-only
    tokio::fs::set_permissions(parent, perms)
        .await
        .expect("Failed to set permissions");

    let config = Config::default();
    let result = config.save_to(&config_path).await;

    // Restore permissions for cleanup
    let mut perms = tokio::fs::metadata(parent)
        .await
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o755);
    let _ = tokio::fs::set_permissions(parent, perms).await;

    // Should fail
    assert!(result.is_err());
}

/// Test Result type alias
#[test]
fn test_result_type_alias() {
    use opensam_config::Result;

    fn returns_result() -> Result<i32> {
        Ok(42)
    }

    fn returns_error() -> Result<i32> {
        let io_err = io::Error::other("test");
        Err(ConfigError::Io(io_err))
    }

    assert_eq!(returns_result().unwrap(), 42);
    assert!(returns_error().is_err());
}
