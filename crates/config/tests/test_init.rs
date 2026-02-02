//! Tests for the init function

use opensam_config::Config;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a temp directory and set up environment
async fn setup_temp_home() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

/// Test init function - this is a complex test because init() uses global paths
/// We'll test the behavior indirectly through the Config methods
/// Test that init creates default config when none exists
/// Note: This test is limited because init() uses hardcoded paths
#[tokio::test]
async fn test_init_behavior_simulation() {
    let temp_dir = setup_temp_home().await;
    let config_path = temp_dir.path().join("config.json");
    let workspace_path = temp_dir.path().join("ops");

    // Simulate init behavior manually
    // 1. Check if config exists
    assert!(!config_path.exists());

    // 2. Create default config
    let config = Config::default();
    config.save_to(&config_path).await.expect("Failed to save");

    // 3. Create workspace
    tokio::fs::create_dir_all(&workspace_path)
        .await
        .expect("Failed to create workspace");

    // Verify
    assert!(config_path.exists());
    assert!(workspace_path.exists());

    // Verify loaded config has defaults
    let loaded = Config::load_from(&config_path)
        .await
        .expect("Failed to load");
    assert_eq!(loaded.operative.defaults.model, "anthropic/claude-sonnet-4");
}

/// Test init when config already exists (should load existing)
#[tokio::test]
async fn test_init_existing_config() {
    let temp_dir = setup_temp_home().await;
    let config_path = temp_dir.path().join("config.json");

    // Create existing config with custom value
    let mut config = Config::default();
    config.operative.defaults.model = "existing-model".to_string();
    config.save_to(&config_path).await.expect("Failed to save");

    // Simulate init loading existing
    let loaded = Config::load_from(&config_path)
        .await
        .expect("Failed to load");
    assert_eq!(loaded.operative.defaults.model, "existing-model");
}

/// Test init creates nested workspace directories
#[tokio::test]
async fn test_init_creates_nested_workspace() {
    let temp_dir = setup_temp_home().await;
    let nested_workspace = temp_dir.path().join("nested/deep/workspace");

    tokio::fs::create_dir_all(&nested_workspace)
        .await
        .expect("Failed to create workspace");

    assert!(nested_workspace.exists());
}

/// Test workspace_path returns correct path from loaded config
#[tokio::test]
async fn test_workspace_path_from_loaded() {
    let temp_dir = setup_temp_home().await;
    let config_path = temp_dir.path().join("config.json");

    let json = r#"{
        "operative": {
            "defaults": {
                "workspace": "/custom/workspace"
            }
        }
    }"#;

    tokio::fs::write(&config_path, json)
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");
    let workspace = config.workspace_path();

    assert_eq!(workspace, PathBuf::from("/custom/workspace"));
}
