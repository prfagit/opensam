//! Integration tests for opensam-bus message types
//!
//! Tests cover:
//! - Message creation and builder patterns
//! - Session key generation
//! - Serialization/deserialization
//! - Edge cases and complex metadata

use opensam_bus::{InboundMessage, OutboundMessage};
use serde_json::json;

// ============================================================================
// InboundMessage Tests
// ============================================================================

#[test]
fn test_inbound_message_creation() {
    let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello World");

    assert_eq!(msg.channel, "telegram");
    assert_eq!(msg.sender_id, "user123");
    assert_eq!(msg.chat_id, "chat456");
    assert_eq!(msg.content, "Hello World");
    assert!(msg.media.is_empty());
    assert!(msg.metadata.is_empty());
}

#[test]
fn test_inbound_message_session_key() {
    // Standard case
    let msg = InboundMessage::new("channel", "sender", "chat", "content");
    assert_eq!(msg.session_key(), "channel:chat");

    // Different channel and chat combinations
    let msg1 = InboundMessage::new("alpha", "s1", "thread-1", "test");
    assert_eq!(msg1.session_key(), "alpha:thread-1");

    let msg2 = InboundMessage::new("secure-channel-v2", "agent-007", "room#general", "test");
    assert_eq!(msg2.session_key(), "secure-channel-v2:room#general");

    // Edge case: empty strings
    let msg3 = InboundMessage::new("", "sender", "", "test");
    assert_eq!(msg3.session_key(), ":");
}

#[test]
fn test_inbound_message_with_media_builder() {
    let msg = InboundMessage::new("radio", "agent-1", "chat-1", "Intel report")
        .with_media("/data/photo1.jpg")
        .with_media("/data/document.pdf")
        .with_media("/data/audio.mp3");

    assert_eq!(msg.media.len(), 3);
    assert_eq!(msg.media[0], "/data/photo1.jpg");
    assert_eq!(msg.media[1], "/data/document.pdf");
    assert_eq!(msg.media[2], "/data/audio.mp3");
}

#[test]
fn test_inbound_message_with_metadata_builder() {
    let msg = InboundMessage::new("radio", "agent-1", "chat-1", "Intel")
        .with_metadata("priority", "high")
        .with_metadata("count", 42)
        .with_metadata("verified", true)
        .with_metadata("ratio", std::f64::consts::PI);

    assert_eq!(msg.metadata.get("priority").unwrap(), &json!("high"));
    assert_eq!(msg.metadata.get("count").unwrap(), &json!(42));
    assert_eq!(msg.metadata.get("verified").unwrap(), &json!(true));
    assert_eq!(
        msg.metadata.get("ratio").unwrap(),
        &json!(std::f64::consts::PI)
    );
}

#[test]
fn test_inbound_message_chained_builders() {
    let msg = InboundMessage::new("secure", "agent-007", "mission-1", "Operation update")
        .with_media("/intel/photo.jpg")
        .with_metadata("classification", "top-secret")
        .with_media("/intel/map.pdf")
        .with_metadata("timestamp", "2024-01-15T10:30:00Z")
        .with_metadata("priority", 1);

    assert_eq!(msg.content, "Operation update");
    assert_eq!(msg.media.len(), 2);
    assert_eq!(msg.metadata.len(), 3);
    assert_eq!(msg.session_key(), "secure:mission-1");
}

// ============================================================================
// OutboundMessage Tests
// ============================================================================

#[test]
fn test_outbound_message_creation() {
    let msg = OutboundMessage::new("telegram", "chat456", "Response message");

    assert_eq!(msg.channel, "telegram");
    assert_eq!(msg.chat_id, "chat456");
    assert_eq!(msg.content, "Response message");
    assert!(msg.reply_to.is_none());
}

#[test]
fn test_outbound_message_reply_to_builder() {
    let msg = OutboundMessage::new("channel", "chat", "Reply").reply_to("original-message-id-123");

    assert_eq!(msg.reply_to, Some("original-message-id-123".to_string()));
}

#[test]
fn test_outbound_message_without_reply() {
    let msg = OutboundMessage::new("channel", "chat", "New message");
    assert!(msg.reply_to.is_none());
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_inbound_message_json_serialization() {
    let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello")
        .with_media("/tmp/photo.jpg")
        .with_metadata("source", "mobile");

    let json_str = serde_json::to_string(&msg).expect("Should serialize to JSON");

    // Verify JSON structure contains expected fields
    assert!(json_str.contains("telegram"));
    assert!(json_str.contains("user123"));
    assert!(json_str.contains("chat456"));
    assert!(json_str.contains("Hello"));
    assert!(json_str.contains("/tmp/photo.jpg"));
    assert!(json_str.contains("source"));
    assert!(json_str.contains("mobile"));
}

#[test]
fn test_inbound_message_json_deserialization() {
    let json_data = r#"{
        "channel": "telegram",
        "sender_id": "user123",
        "chat_id": "chat456",
        "content": "Hello World",
        "timestamp": "2024-01-15T10:30:00.000Z",
        "media": ["/tmp/photo.jpg", "/tmp/doc.pdf"],
        "metadata": {"priority": "high", "count": 5}
    }"#;

    let msg: InboundMessage = serde_json::from_str(json_data).expect("Should deserialize");

    assert_eq!(msg.channel, "telegram");
    assert_eq!(msg.sender_id, "user123");
    assert_eq!(msg.chat_id, "chat456");
    assert_eq!(msg.content, "Hello World");
    assert_eq!(msg.media.len(), 2);
    assert_eq!(msg.metadata.get("priority").unwrap(), &json!("high"));
}

#[test]
fn test_outbound_message_json_serialization() {
    let msg = OutboundMessage::new("telegram", "chat456", "Reply message").reply_to("msg-123");

    let json_str = serde_json::to_string(&msg).expect("Should serialize to JSON");

    assert!(json_str.contains("telegram"));
    assert!(json_str.contains("chat456"));
    assert!(json_str.contains("Reply message"));
    assert!(json_str.contains("msg-123"));
    assert!(json_str.contains("reply_to"));
}

#[test]
fn test_outbound_message_reply_to_skipped_when_none() {
    let msg = OutboundMessage::new("telegram", "chat456", "New message");

    let json_str = serde_json::to_string(&msg).expect("Should serialize to JSON");

    // reply_to should be skipped when None
    assert!(!json_str.contains("reply_to"));
}

#[test]
fn test_outbound_message_default_fields() {
    // Test that media and metadata have default empty values when not provided
    let json_data = r#"{
        "channel": "telegram",
        "chat_id": "chat456",
        "content": "Simple message"
    }"#;

    let msg: OutboundMessage = serde_json::from_str(json_data).expect("Should deserialize");

    assert!(msg.media.is_empty());
    assert!(msg.metadata.is_empty());
    assert!(msg.reply_to.is_none());
}

#[test]
fn test_full_roundtrip_serialization() {
    let original = InboundMessage::new("secure", "agent-007", "mission-x", "Classified intel")
        .with_media("/intel/photo.jpg")
        .with_media("/intel/coordinates.json")
        .with_metadata("clearance", "top-secret")
        .with_metadata("priority", 1);

    let json_str = serde_json::to_string(&original).expect("Should serialize");
    let deserialized: InboundMessage = serde_json::from_str(&json_str).expect("Should deserialize");

    assert_eq!(deserialized.channel, original.channel);
    assert_eq!(deserialized.sender_id, original.sender_id);
    assert_eq!(deserialized.chat_id, original.chat_id);
    assert_eq!(deserialized.content, original.content);
    assert_eq!(deserialized.media, original.media);
    assert_eq!(deserialized.metadata, original.metadata);
}

// ============================================================================
// Complex Metadata Tests
// ============================================================================

#[test]
fn test_nested_metadata_structures() {
    #[derive(serde::Serialize)]
    struct Location {
        lat: f64,
        lng: f64,
    }

    #[derive(serde::Serialize)]
    struct AgentInfo {
        code_name: String,
        clearance_level: i32,
    }

    let msg = InboundMessage::new("secure", "handler", "mission-1", "Agent info")
        .with_metadata(
            "location",
            Location {
                lat: 51.5074,
                lng: -0.1278,
            },
        )
        .with_metadata(
            "agent",
            AgentInfo {
                code_name: "007".to_string(),
                clearance_level: 10,
            },
        )
        .with_metadata("tags", vec!["urgent", "field-op", "europe"]);

    let location = msg.metadata.get("location").unwrap();
    assert_eq!(location.get("lat").unwrap().as_f64().unwrap(), 51.5074);
    assert_eq!(location.get("lng").unwrap().as_f64().unwrap(), -0.1278);

    let tags = msg.metadata.get("tags").unwrap().as_array().unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0], "urgent");
}

#[test]
fn test_metadata_with_json_value() {
    let msg = InboundMessage::new("channel", "sender", "chat", "content")
        .with_metadata("null_field", serde_json::Value::Null)
        .with_metadata("bool_true", serde_json::Value::Bool(true))
        .with_metadata("number", json!(42.5))
        .with_metadata("object", json!({"nested": "value", "count": 10}));

    assert!(msg.metadata.get("null_field").unwrap().is_null());
    assert_eq!(msg.metadata.get("bool_true").unwrap(), &json!(true));
    assert_eq!(msg.metadata.get("number").unwrap(), &json!(42.5));

    let obj = msg.metadata.get("object").unwrap();
    assert_eq!(obj.get("nested").unwrap(), &json!("value"));
    assert_eq!(obj.get("count").unwrap(), &json!(10));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_strings() {
    let inbound = InboundMessage::new("", "", "", "");
    assert_eq!(inbound.session_key(), ":");

    let outbound = OutboundMessage::new("", "", "");
    let json_str = serde_json::to_string(&outbound).unwrap();
    assert!(json_str.contains("\"channel\":\"\""));
}

#[test]
fn test_unicode_and_special_characters() {
    let unicode = "Hello 世界 🌍 Привет \\n\\t\"quoted\"";
    let msg = InboundMessage::new("📡", "👤用户", "💬聊天", unicode);

    let json_str = serde_json::to_string(&msg).unwrap();
    let deserialized: InboundMessage = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.channel, "📡");
    assert_eq!(deserialized.sender_id, "👤用户");
    assert_eq!(deserialized.content, unicode);
}

#[test]
fn test_large_content() {
    let large_content = "x".repeat(10000);
    let msg = InboundMessage::new("channel", "sender", "chat", &large_content);

    let json_str = serde_json::to_string(&msg).unwrap();
    let deserialized: InboundMessage = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.content.len(), 10000);
    assert_eq!(deserialized.content, large_content);
}

#[test]
fn test_many_media_files() {
    let mut msg = InboundMessage::new("channel", "sender", "chat", "Many attachments");

    for i in 0..50 {
        msg = msg.with_media(format!("/path/to/file{}.jpg", i));
    }

    assert_eq!(msg.media.len(), 50);
    assert_eq!(msg.media[25], "/path/to/file25.jpg");
}

#[test]
fn test_many_metadata_entries() {
    let mut msg = InboundMessage::new("channel", "sender", "chat", "Rich metadata");

    for i in 0..100 {
        msg = msg.with_metadata(format!("key_{}", i), i);
    }

    assert_eq!(msg.metadata.len(), 100);
    assert_eq!(msg.metadata.get("key_50").unwrap(), &json!(50));
}

#[test]
fn test_clone_messages() {
    let original = InboundMessage::new("channel", "sender", "chat", "Original")
        .with_media("/file.jpg")
        .with_metadata("key", "value");

    let cloned = original.clone();

    assert_eq!(cloned.channel, original.channel);
    assert_eq!(cloned.content, original.content);
    assert_eq!(cloned.media, original.media);
    assert_eq!(cloned.metadata, original.metadata);

    // Verify deep clone - modifications don't affect original
    let mut cloned = cloned;
    cloned.content = "Modified".to_string();
    assert_ne!(original.content, cloned.content);
}
