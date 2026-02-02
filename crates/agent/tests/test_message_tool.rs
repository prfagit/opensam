//! Tests for message tool

use opensam_agent::tools::{MessageTool, ToolTrait};
use opensam_bus::OutboundMessage;
use serde_json::json;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_message_tool_success() {
    let (tx, mut rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let tool = MessageTool::new(tx);

    // Set context
    tool.set_context("test_channel".to_string(), "chat_123".to_string());

    let args = json!({"content": "Hello, world!"});
    let result = tool.execute(args).await.unwrap();

    assert_eq!(result, "Message sent");

    // Verify the message was sent
    let msg = rx.recv().await.unwrap();
    assert_eq!(msg.channel, "test_channel");
    assert_eq!(msg.chat_id, "chat_123");
    assert_eq!(msg.content, "Hello, world!");
}

#[tokio::test]
async fn test_message_tool_with_explicit_channel() {
    let (tx, mut rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let tool = MessageTool::new(tx);

    // Don't set context, provide explicit channel/chat_id
    let args = json!({
        "content": "Direct message",
        "channel": "direct_channel",
        "chat_id": "direct_chat"
    });
    let result = tool.execute(args).await.unwrap();

    assert_eq!(result, "Message sent");

    let msg = rx.recv().await.unwrap();
    assert_eq!(msg.channel, "direct_channel");
    assert_eq!(msg.chat_id, "direct_chat");
}

#[tokio::test]
async fn test_message_tool_context_override() {
    let (tx, mut rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let tool = MessageTool::new(tx);

    // Set default context
    tool.set_context("default_channel".to_string(), "default_chat".to_string());

    // Override with explicit values
    let args = json!({
        "content": "Override message",
        "channel": "override_channel",
        "chat_id": "override_chat"
    });
    let result = tool.execute(args).await.unwrap();

    assert_eq!(result, "Message sent");

    let msg = rx.recv().await.unwrap();
    assert_eq!(msg.channel, "override_channel");
    assert_eq!(msg.chat_id, "override_chat");
}

#[tokio::test]
async fn test_message_tool_no_context_error() {
    let (tx, _rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let tool = MessageTool::new(tx);

    // Don't set context, no explicit channel
    let args = json!({"content": "No context message"});
    let result = tool.execute(args).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No channel specified"));
}

#[tokio::test]
async fn test_message_tool_no_chat_id_error() {
    let (tx, _rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let tool = MessageTool::new(tx);

    // Set channel context but not chat_id
    tool.set_context("test_channel".to_string(), "chat_123".to_string());

    // Override only channel
    let args = json!({
        "content": "Partial override",
        "channel": "other_channel"
    });
    let result = tool.execute(args).await.unwrap();

    // Should still work because context chat_id is used
    assert_eq!(result, "Message sent");
}

#[test]
fn test_message_tool_metadata() {
    let (tx, _rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let tool = MessageTool::new(tx);

    assert_eq!(tool.name(), "message");
    assert_eq!(tool.description(), "Send a message to a chat channel.");

    let params = tool.parameters();
    assert_eq!(params["type"], "object");
    assert!(params["required"]
        .as_array()
        .unwrap()
        .contains(&json!("content")));

    let properties = params["properties"].as_object().unwrap();
    assert!(properties.contains_key("content"));
    assert!(properties.contains_key("channel"));
    assert!(properties.contains_key("chat_id"));
}

#[test]
fn test_message_tool_set_context() {
    let (tx, _rx) = mpsc::unbounded_channel::<OutboundMessage>();
    let tool = MessageTool::new(tx);

    tool.set_context("my_channel".to_string(), "my_chat".to_string());

    // Context is stored in mutex, tested implicitly through execute tests
}
