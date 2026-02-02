//! Tests for context builder

use opensam_agent::ContextBuilder;
use opensam_provider::Message;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_context_builder_new() {
    let temp_dir = TempDir::new().unwrap();
    let builder = ContextBuilder::new(temp_dir.path());

    // Just verify it doesn't panic
    let _ = builder;
}

#[tokio::test]
async fn test_context_builder_system_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let builder = ContextBuilder::new(temp_dir.path());

    let prompt = builder.build_system_prompt().await;

    // Verify it contains expected sections
    assert!(prompt.contains("opensam"));
    assert!(prompt.contains("Current Time"));
    assert!(prompt.contains("Workspace"));
    assert!(prompt.contains(temp_dir.path().to_str().unwrap()));
}

#[tokio::test]
async fn test_context_builder_with_directive_md() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("DIRECTIVE.md"),
        "# Test Directive\n\nThis is a test.",
    )
    .unwrap();

    let builder = ContextBuilder::new(temp_dir.path());
    let prompt = builder.build_system_prompt().await;

    assert!(prompt.contains("DIRECTIVE.md"));
    assert!(prompt.contains("Test Directive"));
}

#[tokio::test]
async fn test_context_builder_with_memory() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir(temp_dir.path().join("lifepod")).unwrap();
    fs::write(
        temp_dir.path().join("lifepod").join("MEMORY.md"),
        "# Memory\n\nPrevious context here.",
    )
    .unwrap();

    let builder = ContextBuilder::new(temp_dir.path());
    let prompt = builder.build_system_prompt().await;

    assert!(prompt.contains("Memory"));
    assert!(prompt.contains("Previous context here"));
}

#[tokio::test]
async fn test_context_builder_build_messages() {
    let temp_dir = TempDir::new().unwrap();
    let builder = ContextBuilder::new(temp_dir.path());

    let history = vec![Message::user("Previous message")];

    let messages = builder.build_messages(history, "Current message").await;

    // Should have system + history + current
    assert_eq!(messages.len(), 3);

    // First message should be system
    assert_eq!(messages[0].role, "system");
    assert!(messages[0].content.as_deref().unwrap().contains("opensam"));

    // Second message should be previous user message
    assert_eq!(messages[1].role, "user");
    assert_eq!(messages[1].content.as_deref(), Some("Previous message"));

    // Third message should be current user message
    assert_eq!(messages[2].role, "user");
    assert_eq!(messages[2].content.as_deref(), Some("Current message"));
}

#[tokio::test]
async fn test_context_builder_build_messages_empty_history() {
    let temp_dir = TempDir::new().unwrap();
    let builder = ContextBuilder::new(temp_dir.path());

    let messages = builder.build_messages(vec![], "Hello").await;

    // Should have system + current only
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "system");
    assert_eq!(messages[1].role, "user");
    assert_eq!(messages[1].content.as_deref(), Some("Hello"));
}

#[test]
fn test_context_builder_add_tool_result() {
    let temp_dir = TempDir::new().unwrap();
    let _builder = ContextBuilder::new(temp_dir.path());

    let mut messages = vec![Message::system("System"), Message::user("Call a tool")];

    ContextBuilder::add_tool_result(&mut messages, "call_123", "read_file", "file content here");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[2].role, "tool");
    assert_eq!(messages[2].tool_call_id, Some("call_123".to_string()));
    assert_eq!(messages[2].content.as_deref(), Some("file content here"));
}

#[test]
fn test_context_builder_add_assistant_message() {
    let temp_dir = TempDir::new().unwrap();
    let _builder = ContextBuilder::new(temp_dir.path());

    let mut messages = vec![Message::system("System"), Message::user("Question")];

    ContextBuilder::add_assistant_message(&mut messages, Some("My response"), None);

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[2].role, "assistant");
    assert_eq!(messages[2].content.as_deref(), Some("My response"));
    assert!(messages[2].tool_calls.is_none());
}

#[test]
fn test_context_builder_add_assistant_message_with_tool_calls() {
    let temp_dir = TempDir::new().unwrap();
    let _builder = ContextBuilder::new(temp_dir.path());

    let mut messages = vec![Message::system("System"), Message::user("Do something")];

    let tool_calls = vec![opensam_provider::ToolCallDef {
        id: "call_1".to_string(),
        call_type: "function".to_string(),
        function: opensam_provider::FunctionCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "test.txt"}),
        },
    }];

    ContextBuilder::add_assistant_message(&mut messages, Some("I'll help"), Some(tool_calls));

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[2].role, "assistant");
    assert!(messages[2].tool_calls.is_some());

    let calls = messages[2].tool_calls.as_ref().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call_1");
    assert_eq!(calls[0].function.name, "read_file");
}

#[test]
fn test_context_builder_add_assistant_message_empty_content() {
    let temp_dir = TempDir::new().unwrap();
    let _builder = ContextBuilder::new(temp_dir.path());

    let mut messages = vec![Message::system("System")];

    ContextBuilder::add_assistant_message(&mut messages, None, None);

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].role, "assistant");
    assert_eq!(messages[1].content.as_deref(), Some(""));
}
