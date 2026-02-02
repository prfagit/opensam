//! Session Persistence Tests
//!
//! Tests for session history management in the AgentLoop.

use async_trait::async_trait;
use mockall::mock;
use opensam_agent::AgentLoop;
use opensam_bus::{InboundMessage, MessageBus};
use opensam_provider::{ChatParams, ChatResponse, Provider, ProviderError};
use std::path::PathBuf;
use tempfile::TempDir;

// Create a mock implementation of the Provider trait
mock! {
    pub Provider {}

    #[async_trait]
    impl Provider for Provider {
        async fn chat(&self, params: ChatParams) -> Result<ChatResponse, ProviderError>;
        fn default_model(&self) -> String;
        fn is_configured(&self) -> bool;
    }
}

fn create_test_bus() -> MessageBus {
    let (bus, _inbound_rx, _outbound_rx) = MessageBus::channels();
    bus
}

#[tokio::test]
async fn test_generate_session_key() {
    // Test basic session key generation
    let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");
    let key = AgentLoop::<MockProvider>::generate_session_key(&msg);
    assert_eq!(key, "telegram:chat456");
}

#[tokio::test]
async fn test_generate_session_key_different_channels() {
    // Test CLI channel
    let cli_msg = InboundMessage::new("cli", "user", "direct", "Hello CLI");
    let cli_key = AgentLoop::<MockProvider>::generate_session_key(&cli_msg);
    assert_eq!(cli_key, "cli:direct");

    // Test WhatsApp channel
    let wa_msg = InboundMessage::new("whatsapp", "user789", "group123", "Hello WA");
    let wa_key = AgentLoop::<MockProvider>::generate_session_key(&wa_msg);
    assert_eq!(wa_key, "whatsapp:group123");

    // Test Discord channel
    let discord_msg = InboundMessage::new("discord", "bot", "channel-123", "Hello Discord");
    let discord_key = AgentLoop::<MockProvider>::generate_session_key(&discord_msg);
    assert_eq!(discord_key, "discord:channel-123");
}

#[tokio::test]
async fn test_session_history_persisted_across_calls() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let bus = create_test_bus();
    let mut mock = MockProvider::new();

    // First call - expect no history in messages
    mock.expect_chat().times(1).returning(|params| {
        // First call should have: system + user message
        assert_eq!(params.messages.len(), 2);
        assert_eq!(params.messages[1].content.as_deref(), Some("First message"));
        Ok(ChatResponse::text("First response"))
    });

    // Second call - expect history from first call
    mock.expect_chat().times(1).returning(|params| {
        // Second call should have: system + previous user + previous assistant + current user
        assert_eq!(params.messages.len(), 4);
        assert_eq!(params.messages[1].content.as_deref(), Some("First message"));
        assert_eq!(
            params.messages[2].content.as_deref(),
            Some("First response")
        );
        assert_eq!(
            params.messages[3].content.as_deref(),
            Some("Second message")
        );
        Ok(ChatResponse::text("Second response"))
    });

    let agent = AgentLoop::new_with_sessions_dir(
        bus,
        mock,
        PathBuf::from("."),
        "test-model".to_string(),
        5,
        None,
        sessions_dir.clone(),
    );

    // First message
    let msg1 = InboundMessage::new("test", "user1", "chat1", "First message");
    let response1: Option<opensam_bus::OutboundMessage> = agent.process_message(msg1).await;
    assert!(response1.is_some());
    assert_eq!(response1.unwrap().content, "First response");

    // Second message - should have history from first
    let msg2 = InboundMessage::new("test", "user1", "chat1", "Second message");
    let response2: Option<opensam_bus::OutboundMessage> = agent.process_message(msg2).await;
    assert!(response2.is_some());
    assert_eq!(response2.unwrap().content, "Second response");
}

#[tokio::test]
async fn test_session_isolation_between_chats() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let bus = create_test_bus();
    let mut mock = MockProvider::new();

    // First chat should have no history
    mock.expect_chat().times(1).returning(|params| {
        assert_eq!(params.messages.len(), 2); // system + user
        assert_eq!(
            params.messages[1].content.as_deref(),
            Some("Chat 1 message")
        );
        Ok(ChatResponse::text("Chat 1 response"))
    });

    // Second chat should also have no history (different chat_id)
    mock.expect_chat().times(1).returning(|params| {
        assert_eq!(params.messages.len(), 2); // system + user (no history from chat 1)
        assert_eq!(
            params.messages[1].content.as_deref(),
            Some("Chat 2 message")
        );
        Ok(ChatResponse::text("Chat 2 response"))
    });

    let agent = AgentLoop::new_with_sessions_dir(
        bus,
        mock,
        PathBuf::from("."),
        "test-model".to_string(),
        5,
        None,
        sessions_dir,
    );

    // Message to chat 1
    let msg1 = InboundMessage::new("test", "user1", "chat-1", "Chat 1 message");
    let response1: Option<opensam_bus::OutboundMessage> = agent.process_message(msg1).await;
    assert_eq!(response1.unwrap().content, "Chat 1 response");

    // Message to chat 2 - should not see chat 1's history
    let msg2 = InboundMessage::new("test", "user1", "chat-2", "Chat 2 message");
    let response2: Option<opensam_bus::OutboundMessage> = agent.process_message(msg2).await;
    assert_eq!(response2.unwrap().content, "Chat 2 response");
}

#[tokio::test]
async fn test_session_persistence_to_disk() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let bus = create_test_bus();
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .returning(|_| Ok(ChatResponse::text("Persisted response")));

    {
        let agent = AgentLoop::new_with_sessions_dir(
            bus,
            mock,
            PathBuf::from("."),
            "test-model".to_string(),
            5,
            None,
            sessions_dir.clone(),
        );

        let msg = InboundMessage::new("telegram", "user1", "chat123", "Test message");
        let response: Option<opensam_bus::OutboundMessage> = agent.process_message(msg).await;
        assert!(response.is_some());
    }

    // Verify session file was created
    let session_file = sessions_dir.join("telegram_chat123.json");
    assert!(session_file.exists());

    // Read and verify content
    let content = tokio::fs::read_to_string(&session_file).await.unwrap();
    assert!(content.contains("Test message"));
    assert!(content.contains("Persisted response"));
    assert!(content.contains("telegram:chat123"));
}

#[tokio::test]
async fn test_session_history_loaded_from_disk() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Create a pre-existing session file
    let session_key = "telegram:existing_chat";
    let session_file = sessions_dir.join("telegram_existing_chat.json");
    std::fs::create_dir_all(&sessions_dir).unwrap();

    let session_json = serde_json::json!({
        "key": session_key,
        "messages": [
            {
                "role": "user",
                "content": "Previous message from disk",
                "timestamp": "2024-01-01T00:00:00Z"
            },
            {
                "role": "assistant",
                "content": "Previous response from disk",
                "timestamp": "2024-01-01T00:00:01Z"
            }
        ],
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:01Z",
        "metadata": {}
    });

    tokio::fs::write(&session_file, session_json.to_string())
        .await
        .unwrap();

    let bus = create_test_bus();
    let mut mock = MockProvider::new();

    mock.expect_chat().times(1).returning(|params| {
        // Should have: system + loaded user + loaded assistant + current user
        assert_eq!(params.messages.len(), 4);
        assert_eq!(
            params.messages[1].content.as_deref(),
            Some("Previous message from disk")
        );
        assert_eq!(
            params.messages[2].content.as_deref(),
            Some("Previous response from disk")
        );
        assert_eq!(params.messages[3].content.as_deref(), Some("New message"));
        Ok(ChatResponse::text("New response"))
    });

    let agent = AgentLoop::new_with_sessions_dir(
        bus,
        mock,
        PathBuf::from("."),
        "test-model".to_string(),
        5,
        None,
        sessions_dir,
    );

    let msg = InboundMessage::new("telegram", "user1", "existing_chat", "New message");
    let response: Option<opensam_bus::OutboundMessage> = agent.process_message(msg).await;
    assert_eq!(response.unwrap().content, "New response");
}

#[tokio::test]
async fn test_session_history_max_messages_limit() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    // Create a pre-existing session with many messages
    let session_file = sessions_dir.join("test_chat.json");
    std::fs::create_dir_all(&sessions_dir).unwrap();

    // Create 25 messages (more than the default limit of 20)
    let mut messages = Vec::new();
    for i in 0..25 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        messages.push(serde_json::json!({
            "role": role,
            "content": format!("Message {}", i),
            "timestamp": "2024-01-01T00:00:00Z"
        }));
    }

    let session_json = serde_json::json!({
        "key": "test:chat",
        "messages": messages,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z",
        "metadata": {}
    });

    tokio::fs::write(&session_file, session_json.to_string())
        .await
        .unwrap();

    let bus = create_test_bus();
    let mut mock = MockProvider::new();

    mock.expect_chat().times(1).returning(|params| {
        // Should have: system + max 20 history + current = 22
        assert_eq!(params.messages.len(), 22);
        // First history message should be message 5 (index 5), not message 0
        assert_eq!(params.messages[1].content.as_deref(), Some("Message 5"));
        // Last history message should be message 24
        assert_eq!(params.messages[20].content.as_deref(), Some("Message 24"));
        Ok(ChatResponse::text("Response"))
    });

    let agent = AgentLoop::new_with_sessions_dir(
        bus,
        mock,
        PathBuf::from("."),
        "test-model".to_string(),
        5,
        None,
        sessions_dir,
    );

    let msg = InboundMessage::new("test", "user1", "chat", "New message");
    let response: Option<opensam_bus::OutboundMessage> = agent.process_message(msg).await;
    assert!(response.is_some());
}

#[tokio::test]
async fn test_session_error_still_saves() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let bus = create_test_bus();
    let mut mock = MockProvider::new();

    mock.expect_chat()
        .times(1)
        .returning(|_| Err(ProviderError::Api("LLM API error".to_string())));

    let agent = AgentLoop::new_with_sessions_dir(
        bus,
        mock,
        PathBuf::from("."),
        "test-model".to_string(),
        5,
        None,
        sessions_dir.clone(),
    );

    let msg = InboundMessage::new(
        "telegram",
        "user1",
        "error_chat",
        "Message that causes error",
    );
    let response: Option<opensam_bus::OutboundMessage> = agent.process_message(msg).await;

    // Should still return a response with the error
    assert!(response.is_some());
    assert!(response.unwrap().content.contains("Error"));

    // Verify session was still saved
    let session_file = sessions_dir.join("telegram_error_chat.json");
    assert!(session_file.exists());

    let content = tokio::fs::read_to_string(&session_file).await.unwrap();
    assert!(content.contains("Message that causes error"));
    assert!(content.contains("Error"));
}

#[tokio::test]
async fn test_different_channels_different_sessions() {
    let temp_dir = TempDir::new().unwrap();
    let sessions_dir = temp_dir.path().join("sessions");

    let bus = create_test_bus();
    let mut mock = MockProvider::new();

    // Each chat should be independent
    mock.expect_chat().times(2).returning(|params| {
        Ok(ChatResponse::text(format!(
            "Got {} messages",
            params.messages.len()
        )))
    });

    let agent = AgentLoop::new_with_sessions_dir(
        bus,
        mock,
        PathBuf::from("."),
        "test-model".to_string(),
        5,
        None,
        sessions_dir,
    );

    // Message to telegram chat
    let tg_msg = InboundMessage::new("telegram", "user1", "chat1", "Hello Telegram");
    let _response1: Option<opensam_bus::OutboundMessage> = agent.process_message(tg_msg).await;

    // Message to whatsapp chat with same chat_id
    let wa_msg = InboundMessage::new("whatsapp", "user1", "chat1", "Hello WhatsApp");
    let _response2: Option<opensam_bus::OutboundMessage> = agent.process_message(wa_msg).await;

    // Both should succeed with their own separate sessions
}
