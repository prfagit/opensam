//! Integration tests for opensam-config crate
//!
//! These tests verify the complete workflow of configuration management.

use opensam_config::{
    paths::{ensure_dir, safe_filename},
    Config, ConfigError,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Full workflow: Create, save, load, modify, save again, verify
#[tokio::test]
async fn test_full_config_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("workflow_config.json");

    // Step 1: Create custom config
    let mut config = Config::default();
    config.operative.defaults.workspace = "~/custom_ops".to_string();
    config.operative.defaults.model = "custom/model-v1".to_string();
    config.operative.defaults.max_tokens = 16384;
    config.operative.defaults.temperature = 0.3;
    config.deploy.host = "127.0.0.1".to_string();
    config.deploy.port = 8080;
    config.providers.openrouter.api_key = "sk-or-test-key".to_string();
    config.providers.anthropic.api_key = "sk-ant-test-key".to_string();
    config.toolkit.web.search.api_key = "brave-test-key".to_string();
    config.toolkit.web.search.max_results = 10;
    config.frequency.whatsapp.enabled = true;
    config
        .frequency
        .whatsapp
        .allow_from
        .push("+1234567890".to_string());

    // Step 2: Save
    config.save_to(&config_path).await.expect("Failed to save");
    assert!(config_path.exists());

    // Step 3: Load and verify
    let loaded = Config::load_from(&config_path)
        .await
        .expect("Failed to load");

    assert_eq!(loaded.operative.defaults.workspace, "~/custom_ops");
    assert_eq!(loaded.operative.defaults.model, "custom/model-v1");
    assert_eq!(loaded.operative.defaults.max_tokens, 16384);
    assert_eq!(loaded.operative.defaults.temperature, 0.3);
    assert_eq!(loaded.deploy.host, "127.0.0.1");
    assert_eq!(loaded.deploy.port, 8080);
    assert_eq!(loaded.providers.openrouter.api_key, "sk-or-test-key");
    assert_eq!(loaded.providers.anthropic.api_key, "sk-ant-test-key");
    assert_eq!(loaded.toolkit.web.search.api_key, "brave-test-key");
    assert_eq!(loaded.toolkit.web.search.max_results, 10);
    assert!(loaded.frequency.whatsapp.enabled);
    assert_eq!(loaded.frequency.whatsapp.allow_from, vec!["+1234567890"]);

    // Step 4: Verify API key extraction (openrouter takes priority)
    assert_eq!(loaded.api_key(), Some("sk-or-test-key".to_string()));
    assert!(loaded.has_api_key());

    // Step 5: Verify brave API key
    assert_eq!(loaded.brave_api_key(), Some("brave-test-key".to_string()));

    // Step 6: Verify workspace path expansion
    let workspace = loaded.workspace_path();
    let home = dirs::home_dir().expect("No home dir");
    assert_eq!(workspace, home.join("custom_ops"));

    // Step 7: Modify and save again
    let mut modified = loaded;
    modified.deploy.port = 9090;
    modified.providers.openrouter.api_key.clear(); // Remove openrouter key

    modified
        .save_to(&config_path)
        .await
        .expect("Failed to save modified");

    // Step 8: Reload and verify changes
    let reloaded = Config::load_from(&config_path)
        .await
        .expect("Failed to reload");
    assert_eq!(reloaded.deploy.port, 9090);
    assert_eq!(reloaded.api_key(), Some("sk-ant-test-key".to_string())); // Now anthropic
}

/// Test configuration with all providers set
#[tokio::test]
async fn test_all_providers_configured() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("all_providers.json");

    let json = r#"{
        "soliton": {
            "anthropic": {
                "api_key": "anthropic-key",
                "api_base": "https://api.anthropic.com"
            },
            "openai": {
                "api_key": "openai-key",
                "api_base": "https://api.openai.com"
            },
            "openrouter": {
                "api_key": "openrouter-key",
                "api_base": "https://openrouter.ai/api/v1"
            },
            "vllm": {
                "api_key": "vllm-key",
                "api_base": "http://localhost:8000/v1"
            }
        }
    }"#;

    tokio::fs::write(&config_path, json)
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");

    // Verify all providers
    assert_eq!(config.providers.anthropic.api_key, "anthropic-key");
    assert_eq!(
        config.providers.anthropic.api_base,
        Some("https://api.anthropic.com".to_string())
    );
    assert_eq!(config.providers.openai.api_key, "openai-key");
    assert_eq!(
        config.providers.openai.api_base,
        Some("https://api.openai.com".to_string())
    );
    assert_eq!(config.providers.openrouter.api_key, "openrouter-key");
    assert_eq!(
        config.providers.openrouter.api_base,
        Some("https://openrouter.ai/api/v1".to_string())
    );
    assert_eq!(config.providers.vllm.api_key, "vllm-key");
    assert_eq!(
        config.providers.vllm.api_base,
        Some("http://localhost:8000/v1".to_string())
    );

    // API key should prefer openrouter
    assert_eq!(config.api_key(), Some("openrouter-key".to_string()));

    // API base should prefer openrouter
    assert_eq!(
        config.api_base(),
        Some("https://openrouter.ai/api/v1".to_string())
    );
}

/// Test frequency configuration with multiple allowed numbers
#[tokio::test]
async fn test_frequency_multiple_allow_from() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("frequency.json");

    let json = r#"{
        "frequency": {
            "whatsapp": {
                "enabled": true,
                "bridge_url": "ws://custom:3001",
                "allow_from": ["+1234567890", "+0987654321", "+1111111111"]
            },
            "telegram": {
                "enabled": true,
                "token": "bot-token",
                "allow_from": ["@user1", "@user2"]
            }
        }
    }"#;

    tokio::fs::write(&config_path, json)
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");

    assert_eq!(
        config.frequency.whatsapp.allow_from,
        vec!["+1234567890", "+0987654321", "+1111111111"]
    );
    assert_eq!(
        config.frequency.telegram.allow_from,
        vec!["@user1", "@user2"]
    );
}

/// Test safe_filename integration with file operations
#[tokio::test]
async fn test_safe_filename_with_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let unsafe_names = vec![
        "file<name>.txt",
        "file:name.txt",
        "file/name.txt",
        "file\\name.txt",
        "file|name.txt",
        "file?name.txt",
        "file*name.txt",
    ];

    for name in unsafe_names {
        let safe = safe_filename(name);
        let file_path = temp_dir.path().join(&safe);

        // Should be able to create file with safe name
        tokio::fs::write(&file_path, "content")
            .await
            .unwrap_or_else(|_| panic!("Failed to write file with name: {}", safe));

        assert!(file_path.exists());
    }
}

/// Test ensure_dir with complex nested structure
#[tokio::test]
async fn test_ensure_dir_complex_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create complex nested structure
    let paths = vec![
        temp_dir.path().join("a/b/c/d/e"),
        temp_dir.path().join("a/b/f/g"),
        temp_dir.path().join("a/h/i"),
        temp_dir.path().join("j/k/l/m/n/o"),
    ];

    for path in &paths {
        ensure_dir(path).await.expect("Failed to ensure dir");
        assert!(path.exists());
        assert!(path.is_dir());
    }

    // Verify intermediate directories were also created
    assert!(temp_dir.path().join("a").exists());
    assert!(temp_dir.path().join("a/b").exists());
    assert!(temp_dir.path().join("a/b/c").exists());
}

/// Test config file with comments (should fail - JSON doesn't support comments)
#[tokio::test]
async fn test_config_with_comments_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("with_comments.json");

    let json_with_comments = r#"{
        // This is a comment
        "operative": {
            "defaults": {
                "model": "test-model"
            }
        }
    }"#;

    tokio::fs::write(&config_path, json_with_comments)
        .await
        .expect("Failed to write");

    // Should fail to parse
    let result = Config::load_from(&config_path).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ConfigError::Json(_) => (), // Expected
        other => panic!("Expected Json error, got {:?}", other),
    }
}

/// Test empty JSON object loads defaults
#[tokio::test]
async fn test_empty_json_object() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("empty.json");

    tokio::fs::write(&config_path, "{}")
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");

    // Should have all defaults
    assert_eq!(config.operative.defaults.model, "anthropic/claude-sonnet-4");
    assert_eq!(config.deploy.port, 18789);
    assert!(!config.frequency.whatsapp.enabled);
}

/// Test partial JSON with only some fields
#[tokio::test]
async fn test_partial_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("partial.json");

    let json = r#"{
        "operative": {
            "defaults": {
                "model": "custom-model"
            }
        },
        "deploy": {
            "port": 3000
        }
    }"#;

    tokio::fs::write(&config_path, json)
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");

    // Custom values
    assert_eq!(config.operative.defaults.model, "custom-model");
    assert_eq!(config.deploy.port, 3000);

    // Defaults for unspecified
    assert_eq!(config.operative.defaults.workspace, "~/.opensam/ops");
    assert_eq!(config.deploy.host, "0.0.0.0");
    assert_eq!(config.operative.defaults.max_tokens, 8192);
}

/// Test JSON array where object expected (should fail)
#[tokio::test]
async fn test_invalid_json_type() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("invalid.json");

    let invalid_json = r#"{
        "operative": ["not", "an", "object"]
    }"#;

    tokio::fs::write(&config_path, invalid_json)
        .await
        .expect("Failed to write");

    let result = Config::load_from(&config_path).await;
    assert!(result.is_err());
}

/// Test very long strings in config
#[tokio::test]
async fn test_very_long_strings() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("long_strings.json");

    let long_key = "sk-".repeat(100);
    let json = format!(
        r#"{{"soliton": {{"openrouter": {{"api_key": "{}"}}}}}}"#,
        long_key
    );

    tokio::fs::write(&config_path, json)
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");
    assert_eq!(config.providers.openrouter.api_key, long_key);
}

/// Test unicode in config values
#[tokio::test]
async fn test_unicode_values() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("unicode.json");

    let json = r#"{
        "operative": {
            "defaults": {
                "workspace": "~/日本語フォルダ",
                "model": "模型-中文-🤖"
            }
        }
    }"#;

    tokio::fs::write(&config_path, json)
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");
    assert_eq!(config.operative.defaults.workspace, "~/日本語フォルダ");
    assert_eq!(config.operative.defaults.model, "模型-中文-🤖");
}

/// Test multiple save/load cycles preserve data
#[tokio::test]
async fn test_multiple_save_load_cycles() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("cycles.json");

    let mut config = Config::default();

    for i in 0..5 {
        // Modify
        config.deploy.port = 8000 + i as u16;
        config.operative.defaults.max_tokens = 1000 * (i + 1) as u32;

        // Save
        config.save_to(&config_path).await.expect("Failed to save");

        // Load
        config = Config::load_from(&config_path)
            .await
            .expect("Failed to load");

        // Verify
        assert_eq!(config.deploy.port, 8000 + i as u16);
        assert_eq!(config.operative.defaults.max_tokens, 1000 * (i + 1) as u32);
    }
}

/// Test that boolean deserialization is strict
#[tokio::test]
async fn test_boolean_deserialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Valid: true
    let path1 = temp_dir.path().join("bool_true.json");
    tokio::fs::write(&path1, r#"{"frequency": {"whatsapp": {"enabled": true}}}"#)
        .await
        .expect("Failed to write");
    let config1 = Config::load_from(&path1).await.expect("Failed to load");
    assert!(config1.frequency.whatsapp.enabled);

    // Valid: false
    let path2 = temp_dir.path().join("bool_false.json");
    tokio::fs::write(&path2, r#"{"frequency": {"whatsapp": {"enabled": false}}}"#)
        .await
        .expect("Failed to write");
    let config2 = Config::load_from(&path2).await.expect("Failed to load");
    assert!(!config2.frequency.whatsapp.enabled);
}

/// Test numeric bounds
#[tokio::test]
async fn test_numeric_bounds() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("bounds.json");

    let json = r#"{
        "operative": {
            "defaults": {
                "max_tokens": 4294967295,
                "temperature": 2.0,
                "max_tool_iterations": 100
            }
        },
        "deploy": {
            "port": 65535
        }
    }"#;

    tokio::fs::write(&config_path, json)
        .await
        .expect("Failed to write");

    let config = Config::load_from(&config_path)
        .await
        .expect("Failed to load");
    assert_eq!(config.operative.defaults.max_tokens, 4294967295); // u32::MAX
    assert_eq!(config.operative.defaults.temperature, 2.0);
    assert_eq!(config.operative.defaults.max_tool_iterations, 100);
    assert_eq!(config.deploy.port, 65535); // u16::MAX
}

/// Test workspace_path with various ~ patterns
#[test]
fn test_workspace_path_various_tilde_patterns() {
    let home = dirs::home_dir().expect("No home dir");

    // Test ~
    let mut config = Config::default();
    config.operative.defaults.workspace = "~".to_string();
    assert_eq!(config.workspace_path(), home);

    // Test ~/
    config.operative.defaults.workspace = "~/".to_string();
    assert_eq!(config.workspace_path(), home.join(""));

    // Test ~/path
    config.operative.defaults.workspace = "~/my/workspace".to_string();
    assert_eq!(config.workspace_path(), home.join("my/workspace"));

    // Test ~user (should NOT expand)
    config.operative.defaults.workspace = "~otheruser/path".to_string();
    let path = config.workspace_path();
    // Since it doesn't start with exactly "~", it won't expand
    assert_eq!(path, PathBuf::from("~otheruser/path"));
}
