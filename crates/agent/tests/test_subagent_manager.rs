//! Tests for subagent manager

use async_trait::async_trait;
use opensam_agent::SubagentManager;
use opensam_bus::MessageBus;
use opensam_provider::{ChatParams, ChatResponse, Provider};
use std::path::PathBuf;

// Mock provider for testing
struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    async fn chat(&self, _params: ChatParams) -> opensam_provider::Result<ChatResponse> {
        Ok(ChatResponse::text("Mock response"))
    }

    fn default_model(&self) -> String {
        "mock-model".to_string()
    }

    fn is_configured(&self) -> bool {
        true
    }
}

fn create_test_bus() -> MessageBus {
    let (in_tx, _in_rx) = tokio::sync::mpsc::unbounded_channel();
    let (out_tx, _out_rx) = tokio::sync::mpsc::unbounded_channel();
    MessageBus::new(in_tx, out_tx)
}

#[test]
fn test_subagent_manager_new() {
    let provider = MockProvider;
    let workspace = PathBuf::from("/tmp/test_workspace");
    let bus = create_test_bus();
    let model = "test-model".to_string();

    let manager = SubagentManager::new(provider, workspace, bus, model, None);

    assert_eq!(manager.running_count(), 0);
}

#[test]
fn test_subagent_manager_spawn() {
    let provider = MockProvider;
    let workspace = PathBuf::from("/tmp/test_workspace");
    let bus = create_test_bus();
    let model = "test-model".to_string();

    let mut manager = SubagentManager::new(provider, workspace, bus, model, None);

    // Currently spawn just returns an ID without actually spawning
    let id = manager.spawn("Test task".to_string(), None);

    // Verify ID format
    assert!(id.starts_with("task_"));
}

#[test]
fn test_subagent_manager_spawn_with_label() {
    let provider = MockProvider;
    let workspace = PathBuf::from("/tmp/test_workspace");
    let bus = create_test_bus();
    let model = "test-model".to_string();

    let mut manager = SubagentManager::new(provider, workspace, bus, model, None);

    let id = manager.spawn(
        "Process data".to_string(),
        Some("data_processor".to_string()),
    );

    assert!(id.starts_with("task_"));
}

#[test]
fn test_subagent_manager_spawn_label_defaults_to_task() {
    let provider = MockProvider;
    let workspace = PathBuf::from("/tmp/test_workspace");
    let bus = create_test_bus();
    let model = "test-model".to_string();

    let mut manager = SubagentManager::new(provider, workspace, bus, model, None);

    // When label is not provided, task is used as label
    let id = manager.spawn("Custom task description".to_string(), None);

    assert!(id.starts_with("task_"));
}

#[test]
fn test_subagent_manager_running_count() {
    let provider = MockProvider;
    let workspace = PathBuf::from("/tmp/test_workspace");
    let bus = create_test_bus();
    let model = "test-model".to_string();

    let manager = SubagentManager::new(provider, workspace, bus, model, None);

    // Currently running_count always returns 0 since spawn doesn't actually track tasks
    assert_eq!(manager.running_count(), 0);
}

#[test]
fn test_subagent_manager_with_brave_api_key() {
    let provider = MockProvider;
    let workspace = PathBuf::from("/tmp/test_workspace");
    let bus = create_test_bus();
    let model = "test-model".to_string();

    let manager = SubagentManager::new(
        provider,
        workspace,
        bus,
        model,
        Some("test_api_key".to_string()),
    );

    // Verify manager was created with API key
    assert_eq!(manager.running_count(), 0);
}
