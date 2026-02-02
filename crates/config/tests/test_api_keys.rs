//! Tests for API key extraction and provider selection logic

use opensam_config::{Config, ProviderConfig};

/// Test api_key returns None when no keys are set
#[test]
fn test_api_key_none_when_empty() {
    let config = Config::default();
    assert_eq!(config.api_key(), None);
}

/// Test api_key returns openrouter key when set
#[test]
fn test_api_key_prefers_openrouter() {
    let mut config = Config::default();
    config.providers.openrouter.api_key = "openrouter-key".to_string();

    assert_eq!(config.api_key(), Some("openrouter-key".to_string()));
}

/// Test api_key returns anthropic key when openrouter not set
#[test]
fn test_api_key_falls_back_to_anthropic() {
    let mut config = Config::default();
    config.providers.anthropic.api_key = "anthropic-key".to_string();

    assert_eq!(config.api_key(), Some("anthropic-key".to_string()));
}

/// Test api_key returns openai key when openrouter and anthropic not set
#[test]
fn test_api_key_falls_back_to_openai() {
    let mut config = Config::default();
    config.providers.openai.api_key = "openai-key".to_string();

    assert_eq!(config.api_key(), Some("openai-key".to_string()));
}

/// Test api_key returns vllm key when others not set
#[test]
fn test_api_key_falls_back_to_vllm() {
    let mut config = Config::default();
    config.providers.vllm.api_key = "vllm-key".to_string();

    assert_eq!(config.api_key(), Some("vllm-key".to_string()));
}

/// Test api_key prioritization: openrouter > anthropic > openai > vllm
#[test]
fn test_api_key_priority_order() {
    let mut config = Config::default();

    // Set all keys
    config.providers.openrouter.api_key = "openrouter".to_string();
    config.providers.anthropic.api_key = "anthropic".to_string();
    config.providers.openai.api_key = "openai".to_string();
    config.providers.vllm.api_key = "vllm".to_string();

    // Should prefer openrouter
    assert_eq!(config.api_key(), Some("openrouter".to_string()));

    // Remove openrouter, should prefer anthropic
    config.providers.openrouter.api_key = "".to_string();
    assert_eq!(config.api_key(), Some("anthropic".to_string()));

    // Remove anthropic, should prefer openai
    config.providers.anthropic.api_key = "".to_string();
    assert_eq!(config.api_key(), Some("openai".to_string()));

    // Remove openai, should prefer vllm
    config.providers.openai.api_key = "".to_string();
    assert_eq!(config.api_key(), Some("vllm".to_string()));

    // Remove all, should return None
    config.providers.vllm.api_key = "".to_string();
    assert_eq!(config.api_key(), None);
}

/// Test has_api_key returns true when any key is set
#[test]
fn test_has_api_key_true_when_any_set() {
    let mut config = Config::default();

    config.providers.openrouter.api_key = "key".to_string();
    assert!(config.has_api_key());

    config.providers.openrouter.api_key = "".to_string();
    config.providers.anthropic.api_key = "key".to_string();
    assert!(config.has_api_key());

    config.providers.anthropic.api_key = "".to_string();
    config.providers.openai.api_key = "key".to_string();
    assert!(config.has_api_key());

    config.providers.openai.api_key = "".to_string();
    config.providers.vllm.api_key = "key".to_string();
    assert!(config.has_api_key());
}

/// Test has_api_key returns false when no keys set
#[test]
fn test_has_api_key_false_when_none_set() {
    let config = Config::default();
    assert!(!config.has_api_key());
}

/// Test api_base returns None when no provider configured
#[test]
fn test_api_base_none_when_no_provider() {
    let config = Config::default();
    assert_eq!(config.api_base(), None);
}

/// Test api_base returns openrouter base when openrouter configured
#[test]
fn test_api_base_prefers_openrouter() {
    let mut config = Config::default();
    config.providers.openrouter.api_key = "key".to_string();

    // Should return default openrouter base
    assert_eq!(
        config.api_base(),
        Some("https://openrouter.ai/api/v1".to_string())
    );
}

/// Test api_base returns custom openrouter base when set
#[test]
fn test_api_base_custom_openrouter() {
    let mut config = Config::default();
    config.providers.openrouter.api_key = "key".to_string();
    config.providers.openrouter.api_base = Some("https://custom.openrouter.com".to_string());

    assert_eq!(
        config.api_base(),
        Some("https://custom.openrouter.com".to_string())
    );
}

/// Test api_base returns vllm base when vllm configured
#[test]
fn test_api_base_returns_vllm() {
    let mut config = Config::default();
    config.providers.vllm.api_key = "key".to_string();
    config.providers.vllm.api_base = Some("http://localhost:8000/v1".to_string());

    assert_eq!(
        config.api_base(),
        Some("http://localhost:8000/v1".to_string())
    );
}

/// Test api_base prioritizes openrouter over vllm
#[test]
fn test_api_base_prioritizes_openrouter() {
    let mut config = Config::default();

    // Configure both
    config.providers.openrouter.api_key = "openrouter-key".to_string();
    config.providers.vllm.api_key = "vllm-key".to_string();
    config.providers.vllm.api_base = Some("http://vllm.local".to_string());

    // Should prefer openrouter
    assert_eq!(
        config.api_base(),
        Some("https://openrouter.ai/api/v1".to_string())
    );
}

/// Test brave_api_key returns None when not set
#[test]
fn test_brave_api_key_none() {
    let config = Config::default();
    assert_eq!(config.brave_api_key(), None);
}

/// Test brave_api_key returns key when set
#[test]
fn test_brave_api_key_some() {
    let mut config = Config::default();
    config.toolkit.web.search.api_key = "brave-api-key".to_string();

    assert_eq!(config.brave_api_key(), Some("brave-api-key".to_string()));
}

/// Test brave_api_key returns None for empty string
#[test]
fn test_brave_api_key_empty_string() {
    let mut config = Config::default();
    config.toolkit.web.search.api_key = "".to_string();

    assert_eq!(config.brave_api_key(), None);
}

/// Test api_key returns None for empty string (not just unset)
#[test]
fn test_api_key_empty_string_treated_as_none() {
    let mut config = Config::default();
    config.providers.anthropic.api_key = "".to_string();

    assert_eq!(config.api_key(), None);
}

/// Test ProviderConfig with api_base serialization roundtrip
#[test]
fn test_provider_config_roundtrip() {
    let provider = ProviderConfig {
        api_key: "test-key".to_string(),
        api_base: Some("https://api.example.com".to_string()),
    };

    let json = serde_json::to_string(&provider).expect("Failed to serialize");
    let deserialized: ProviderConfig = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.api_key, "test-key");
    assert_eq!(
        deserialized.api_base,
        Some("https://api.example.com".to_string())
    );
}

/// Test ProviderConfig with None api_base serialization
#[test]
fn test_provider_config_none_base_roundtrip() {
    let provider = ProviderConfig {
        api_key: "test-key".to_string(),
        api_base: None,
    };

    let json = serde_json::to_string(&provider).expect("Failed to serialize");
    let deserialized: ProviderConfig = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.api_key, "test-key");
    assert_eq!(deserialized.api_base, None);
}

/// Test full config API key extraction from JSON
#[tokio::test]
async fn test_api_key_from_deserialized_config() {
    let json = r#"{
        "soliton": {
            "openrouter": {
                "api_key": "test-key"
            }
        }
    }"#;

    let config: Config = serde_json::from_str(json).expect("Failed to parse");

    assert!(config.has_api_key());
    assert_eq!(config.api_key(), Some("test-key".to_string()));
}

/// Test api_base when vllm has empty api_base
#[test]
fn test_api_base_vllm_empty_base() {
    let mut config = Config::default();
    config.providers.vllm.api_key = "key".to_string();
    config.providers.vllm.api_base = Some("".to_string());

    // Empty api_base is treated as None (invalid/unset)
    assert_eq!(config.api_base(), None);
}
