//! Tests for Config serialization, deserialization, and core functionality

use opensam_config::{
    Config, DeployConfig, FrequencyConfig, OperativeConfig, OperativeDefaults, ProviderConfig,
    SolitonConfig, TelegramConfig, ToolkitConfig, WebSearchConfig, WebToolkitConfig,
    WhatsAppConfig,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a temporary directory for tests
fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Test that default Config has expected values
#[test]
fn test_config_defaults() {
    let config = Config::default();

    // Operative defaults
    assert_eq!(config.operative.defaults.workspace, "~/.opensam/ops");
    assert_eq!(config.operative.defaults.model, "anthropic/claude-sonnet-4");
    assert_eq!(config.operative.defaults.max_tokens, 8192);
    assert_eq!(config.operative.defaults.temperature, 0.7);
    assert_eq!(config.operative.defaults.max_tool_iterations, 20);

    // Deploy defaults
    assert_eq!(config.deploy.host, "0.0.0.0");
    assert_eq!(config.deploy.port, 18789);

    // Frequency defaults
    assert!(!config.frequency.whatsapp.enabled);
    assert_eq!(config.frequency.whatsapp.bridge_url, "ws://localhost:3001");
    assert!(config.frequency.whatsapp.allow_from.is_empty());
    assert!(!config.frequency.telegram.enabled);
    assert!(config.frequency.telegram.token.is_empty());
    assert!(config.frequency.telegram.allow_from.is_empty());

    // Toolkit defaults
    assert_eq!(config.toolkit.web.search.max_results, 5);
    assert!(config.toolkit.web.search.api_key.is_empty());

    // Provider defaults (all empty)
    assert!(config.providers.anthropic.api_key.is_empty());
    assert!(config.providers.anthropic.api_base.is_none());
    assert!(config.providers.openai.api_key.is_empty());
    assert!(config.providers.openai.api_base.is_none());
    assert!(config.providers.openrouter.api_key.is_empty());
    assert!(config.providers.openrouter.api_base.is_none());
    assert!(config.providers.vllm.api_key.is_empty());
    assert!(config.providers.vllm.api_base.is_none());
}

/// Test ProviderConfig defaults
#[test]
fn test_provider_config_defaults() {
    let provider = ProviderConfig::default();
    assert!(provider.api_key.is_empty());
    assert_eq!(provider.api_base, None);
}

/// Test SolitonConfig defaults
#[test]
fn test_soliton_config_defaults() {
    let soliton = SolitonConfig::default();
    assert!(soliton.anthropic.api_key.is_empty());
    assert!(soliton.openai.api_key.is_empty());
    assert!(soliton.openrouter.api_key.is_empty());
    assert!(soliton.vllm.api_key.is_empty());
}

/// Test WhatsAppConfig defaults
#[test]
fn test_whatsapp_config_defaults() {
    let whatsapp = WhatsAppConfig::default();
    assert!(!whatsapp.enabled);
    assert_eq!(whatsapp.bridge_url, "ws://localhost:3001");
    assert!(whatsapp.allow_from.is_empty());
}

/// Test TelegramConfig defaults
#[test]
fn test_telegram_config_defaults() {
    let telegram = TelegramConfig::default();
    assert!(!telegram.enabled);
    assert!(telegram.token.is_empty());
    assert!(telegram.allow_from.is_empty());
}

/// Test FrequencyConfig defaults
#[test]
fn test_frequency_config_defaults() {
    let freq = FrequencyConfig::default();
    assert!(!freq.whatsapp.enabled);
    assert!(!freq.telegram.enabled);
}

/// Test OperativeDefaults
#[test]
fn test_operative_defaults() {
    let defaults = OperativeDefaults::default();
    assert_eq!(defaults.workspace, "~/.opensam/ops");
    assert_eq!(defaults.model, "anthropic/claude-sonnet-4");
    assert_eq!(defaults.max_tokens, 8192);
    assert_eq!(defaults.temperature, 0.7);
    assert_eq!(defaults.max_tool_iterations, 20);
}

/// Test OperativeConfig defaults
#[test]
fn test_operative_config_defaults() {
    let op = OperativeConfig::default();
    assert_eq!(op.defaults.workspace, "~/.opensam/ops");
}

/// Test WebSearchConfig defaults
#[test]
fn test_web_search_config_defaults() {
    let search = WebSearchConfig::default();
    assert!(search.api_key.is_empty());
    assert_eq!(search.max_results, 5);
}

/// Test WebToolkitConfig defaults
#[test]
fn test_web_toolkit_config_defaults() {
    let web = WebToolkitConfig::default();
    assert_eq!(web.search.max_results, 5);
}

/// Test ToolkitConfig defaults
#[test]
fn test_toolkit_config_defaults() {
    let toolkit = ToolkitConfig::default();
    assert_eq!(toolkit.web.search.max_results, 5);
}

/// Test DeployConfig defaults
#[test]
fn test_deploy_config_defaults() {
    let deploy = DeployConfig::default();
    assert_eq!(deploy.host, "0.0.0.0");
    assert_eq!(deploy.port, 18789);
}

/// Test Config serialization to JSON
#[test]
fn test_config_serialization() {
    let config = Config::default();
    let json = serde_json::to_string(&config).expect("Failed to serialize");

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Invalid JSON");

    // Check structure
    assert!(parsed.get("operative").is_some());
    assert!(parsed.get("frequency").is_some());
    assert!(parsed.get("soliton").is_some());
    assert!(parsed.get("deploy").is_some());
    assert!(parsed.get("toolkit").is_some());
}

/// Test Config deserialization from JSON
#[test]
fn test_config_deserialization() {
    let json = r#"{
        "operative": {
            "defaults": {
                "workspace": "~/custom/workspace",
                "model": "custom/model",
                "max_tokens": 4096,
                "temperature": 0.5,
                "max_tool_iterations": 10
            }
        },
        "frequency": {
            "whatsapp": {
                "enabled": true,
                "bridge_url": "ws://custom:3001",
                "allow_from": ["+1234567890"]
            },
            "telegram": {
                "enabled": true,
                "token": "bot-token-123",
                "allow_from": ["@username"]
            }
        },
        "soliton": {
            "anthropic": {
                "api_key": "anthropic-key",
                "api_base": "https://custom.anthropic.com"
            },
            "openai": {
                "api_key": "openai-key"
            }
        },
        "deploy": {
            "host": "127.0.0.1",
            "port": 8080
        },
        "toolkit": {
            "web": {
                "search": {
                    "api_key": "brave-key",
                    "max_results": 10
                }
            }
        }
    }"#;

    let config: Config = serde_json::from_str(json).expect("Failed to deserialize");

    // Verify operative
    assert_eq!(config.operative.defaults.workspace, "~/custom/workspace");
    assert_eq!(config.operative.defaults.model, "custom/model");
    assert_eq!(config.operative.defaults.max_tokens, 4096);
    assert_eq!(config.operative.defaults.temperature, 0.5);
    assert_eq!(config.operative.defaults.max_tool_iterations, 10);

    // Verify frequency
    assert!(config.frequency.whatsapp.enabled);
    assert_eq!(config.frequency.whatsapp.bridge_url, "ws://custom:3001");
    assert_eq!(config.frequency.whatsapp.allow_from, vec!["+1234567890"]);
    assert!(config.frequency.telegram.enabled);
    assert_eq!(config.frequency.telegram.token, "bot-token-123");
    assert_eq!(config.frequency.telegram.allow_from, vec!["@username"]);

    // Verify providers
    assert_eq!(config.providers.anthropic.api_key, "anthropic-key");
    assert_eq!(
        config.providers.anthropic.api_base,
        Some("https://custom.anthropic.com".to_string())
    );
    assert_eq!(config.providers.openai.api_key, "openai-key");

    // Verify deploy
    assert_eq!(config.deploy.host, "127.0.0.1");
    assert_eq!(config.deploy.port, 8080);

    // Verify toolkit
    assert_eq!(config.toolkit.web.search.api_key, "brave-key");
    assert_eq!(config.toolkit.web.search.max_results, 10);
}

/// Test Config deserialization with missing fields (should use defaults)
#[test]
fn test_config_deserialization_partial() {
    let json = r#"{}"#;
    let config: Config = serde_json::from_str(json).expect("Failed to deserialize");

    // Should use all defaults
    assert_eq!(config.operative.defaults.model, "anthropic/claude-sonnet-4");
    assert_eq!(config.deploy.port, 18789);
    assert!(!config.frequency.whatsapp.enabled);
}

/// Test Config deserialization with partial operative
#[test]
fn test_config_deserialization_partial_operative() {
    let json = r#"{
        "operative": {
            "defaults": {
                "model": "custom-model"
            }
        }
    }"#;

    let config: Config = serde_json::from_str(json).expect("Failed to deserialize");

    // Custom value
    assert_eq!(config.operative.defaults.model, "custom-model");

    // Defaults for other fields
    assert_eq!(config.operative.defaults.workspace, "~/.opensam/ops");
    assert_eq!(config.operative.defaults.max_tokens, 8192);
}

/// Test roundtrip serialization/deserialization
#[test]
fn test_config_roundtrip() {
    let original = Config::default();
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let deserialized: Config = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(
        original.operative.defaults.model,
        deserialized.operative.defaults.model
    );
    assert_eq!(
        original.operative.defaults.max_tokens,
        deserialized.operative.defaults.max_tokens
    );
    assert_eq!(original.deploy.port, deserialized.deploy.port);
}

/// Test Config save and load roundtrip (async)
#[tokio::test]
async fn test_config_save_load_roundtrip() {
    let temp_dir = temp_dir();
    let config_path = temp_dir.path().join("test_config.json");

    // Create config with custom values
    let mut config = Config::default();
    config.operative.defaults.model = "test-model".to_string();
    config.deploy.port = 9999;

    // Save
    config.save_to(&config_path).await.expect("Failed to save");

    // Verify file exists
    assert!(config_path.exists());

    // Load
    let loaded = Config::load_from(&config_path)
        .await
        .expect("Failed to load");

    // Verify values
    assert_eq!(loaded.operative.defaults.model, "test-model");
    assert_eq!(loaded.deploy.port, 9999);
}

/// Test Config load from non-existent path returns default
#[tokio::test]
async fn test_config_load_nonexistent_returns_default() {
    let temp_dir = temp_dir();
    let config_path = temp_dir.path().join("nonexistent.json");

    let config = Config::load_from(&config_path)
        .await
        .expect("Should return default");

    // Should have default values
    assert_eq!(config.operative.defaults.model, "anthropic/claude-sonnet-4");
    assert_eq!(config.deploy.port, 18789);
}

/// Test Config save creates parent directories
#[tokio::test]
async fn test_config_save_creates_directories() {
    let temp_dir = temp_dir();
    let nested_path = temp_dir.path().join("nested/deep/config.json");

    let config = Config::default();
    config.save_to(&nested_path).await.expect("Failed to save");

    assert!(nested_path.exists());
}

/// Test workspace_path expansion with ~
#[test]
fn test_workspace_path_expansion() {
    let mut config = Config::default();
    config.operative.defaults.workspace = "~/custom/path".to_string();

    let path = config.workspace_path();

    // Should expand ~ to home directory
    let home = dirs::home_dir().expect("No home dir");
    let expected = home.join("custom/path");
    assert_eq!(path, expected);
}

/// Test workspace_path without ~
#[test]
fn test_workspace_path_no_expansion() {
    let mut config = Config::default();
    config.operative.defaults.workspace = "/absolute/path".to_string();

    let path = config.workspace_path();
    assert_eq!(path, PathBuf::from("/absolute/path"));
}

/// Test workspace_path with just ~
#[test]
fn test_workspace_path_home_only() {
    let mut config = Config::default();
    config.operative.defaults.workspace = "~".to_string();

    let path = config.workspace_path();

    let home = dirs::home_dir().expect("No home dir");
    assert_eq!(path, home);
}

/// Test default_model helper
#[test]
fn test_default_model_helper() {
    let mut config = Config::default();
    assert_eq!(config.default_model(), "anthropic/claude-sonnet-4");

    config.operative.defaults.model = "custom/model".to_string();
    assert_eq!(config.default_model(), "custom/model");
}

/// Test has_api_key when no keys set
#[test]
fn test_has_api_key_false() {
    let config = Config::default();
    assert!(!config.has_api_key());
}

/// Test brave_api_key when not set
#[test]
fn test_brave_api_key_none() {
    let config = Config::default();
    assert_eq!(config.brave_api_key(), None);
}

/// Test brave_api_key when set
#[test]
fn test_brave_api_key_some() {
    let mut config = Config::default();
    config.toolkit.web.search.api_key = "my-brave-key".to_string();
    assert_eq!(config.brave_api_key(), Some("my-brave-key".to_string()));
}

/// Test that pretty JSON is generated on save (visual verification)
#[tokio::test]
async fn test_config_save_pretty_json() {
    let temp_dir = temp_dir();
    let config_path = temp_dir.path().join("pretty.json");

    let config = Config::default();
    config.save_to(&config_path).await.expect("Failed to save");

    let content = tokio::fs::read_to_string(&config_path)
        .await
        .expect("Failed to read");

    // Should be pretty-printed (contains newlines)
    assert!(content.contains('\n'));

    // Should be valid JSON
    let _: Config = serde_json::from_str(&content).expect("Invalid JSON");
}

/// Test serialization skips None values for api_base
#[test]
fn test_provider_config_skips_none_api_base() {
    let provider = ProviderConfig::default();
    let json = serde_json::to_string(&provider).expect("Failed to serialize");

    // Should not contain api_base when None
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Invalid JSON");
    assert!(parsed.get("api_base").is_none());
}

/// Test serialization includes Some values for api_base
#[test]
fn test_provider_config_includes_some_api_base() {
    let provider = ProviderConfig {
        api_key: "key".to_string(),
        api_base: Some("https://api.example.com".to_string()),
    };
    let json = serde_json::to_string(&provider).expect("Failed to serialize");

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Invalid JSON");
    assert_eq!(parsed["api_base"].as_str(), Some("https://api.example.com"));
}

/// Test complex nested serialization
#[test]
fn test_complex_config_serialization() {
    let json = r#"{
        "soliton": {
            "vllm": {
                "api_key": "vllm-key",
                "api_base": "http://localhost:8000/v1"
            }
        }
    }"#;

    let config: Config = serde_json::from_str(json).expect("Failed to deserialize");
    assert_eq!(config.providers.vllm.api_key, "vllm-key");
    assert_eq!(
        config.providers.vllm.api_base,
        Some("http://localhost:8000/v1".to_string())
    );

    // Serialize back
    let output = serde_json::to_string_pretty(&config).expect("Failed to serialize");
    let reparsed: Config = serde_json::from_str(&output).expect("Failed to re-deserialize");
    assert_eq!(reparsed.providers.vllm.api_key, "vllm-key");
}
