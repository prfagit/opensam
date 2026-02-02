//! Integration tests for opensam-bus MessageBus
//!
//! Tests cover:
//! - Channel creation and initialization
//! - Publishing messages through the bus
//! - Cloning and sharing bus instances
//! - Error handling

use opensam_bus::{InboundMessage, MessageBus, OutboundMessage};

// ============================================================================
// Channel Creation Tests
// ============================================================================

#[test]
fn test_channels_creation() {
    let (bus, in_rx, out_rx) = MessageBus::channels();

    // Just verify the types compile and we can create channels
    // Drop receivers to avoid unused warnings
    drop(in_rx);
    drop(out_rx);
    drop(bus);
}

#[test]
fn test_multiple_channel_pairs() {
    // Create multiple independent channel pairs
    let (bus1, in_rx1, out_rx1) = MessageBus::channels();
    let (bus2, in_rx2, out_rx2) = MessageBus::channels();

    // Verify they're independent by dropping
    drop(bus1);
    drop(bus2);
    drop(in_rx1);
    drop(in_rx2);
    drop(out_rx1);
    drop(out_rx2);
}

// ============================================================================
// Inbound Message Publishing Tests
// ============================================================================

#[tokio::test]
async fn test_publish_single_inbound_message() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");
    bus.publish_inbound(msg)
        .expect("Should publish successfully");

    let received = in_rx.recv().await.expect("Should receive message");
    assert_eq!(received.channel, "telegram");
    assert_eq!(received.sender_id, "user123");
    assert_eq!(received.chat_id, "chat456");
    assert_eq!(received.content, "Hello");
}

#[tokio::test]
async fn test_publish_multiple_inbound_messages() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    let messages = vec![
        InboundMessage::new("ch1", "user1", "chat1", "Message 1"),
        InboundMessage::new("ch2", "user2", "chat2", "Message 2"),
        InboundMessage::new("ch1", "user1", "chat1", "Message 3"),
    ];

    for msg in &messages {
        bus.publish_inbound(msg.clone()).expect("Should publish");
    }

    for expected in &messages {
        let received = in_rx.recv().await.expect("Should receive");
        assert_eq!(received.channel, expected.channel);
        assert_eq!(received.content, expected.content);
    }
}

#[tokio::test]
async fn test_publish_inbound_with_media_and_metadata() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    let msg = InboundMessage::new("secure", "agent", "mission", "Intel")
        .with_media("/intel/photo.jpg")
        .with_metadata("priority", "high");

    bus.publish_inbound(msg).expect("Should publish");

    let received = in_rx.recv().await.expect("Should receive");
    assert_eq!(received.media.len(), 1);
    assert_eq!(received.media[0], "/intel/photo.jpg");
    assert!(received.metadata.contains_key("priority"));
}

// ============================================================================
// Outbound Message Publishing Tests
// ============================================================================

#[tokio::test]
async fn test_publish_single_outbound_message() {
    let (bus, in_rx, mut out_rx) = MessageBus::channels();
    drop(in_rx);

    let msg = OutboundMessage::new("telegram", "chat456", "Response");
    bus.publish_outbound(msg)
        .expect("Should publish successfully");

    let received = out_rx.recv().await.expect("Should receive message");
    assert_eq!(received.channel, "telegram");
    assert_eq!(received.chat_id, "chat456");
    assert_eq!(received.content, "Response");
}

#[tokio::test]
async fn test_publish_outbound_with_reply() {
    let (bus, in_rx, mut out_rx) = MessageBus::channels();
    drop(in_rx);

    let msg =
        OutboundMessage::new("telegram", "chat456", "Reply message").reply_to("original-msg-id");

    bus.publish_outbound(msg).expect("Should publish");

    let received = out_rx.recv().await.expect("Should receive");
    assert_eq!(received.reply_to, Some("original-msg-id".to_string()));
}

#[tokio::test]
async fn test_publish_multiple_outbound_messages() {
    let (bus, in_rx, mut out_rx) = MessageBus::channels();
    drop(in_rx);

    let messages = vec![
        OutboundMessage::new("ch1", "chat1", "Response 1"),
        OutboundMessage::new("ch2", "chat2", "Response 2"),
        OutboundMessage::new("ch1", "chat1", "Response 3"),
    ];

    for msg in &messages {
        bus.publish_outbound(msg.clone()).expect("Should publish");
    }

    for expected in &messages {
        let received = out_rx.recv().await.expect("Should receive");
        assert_eq!(received.channel, expected.channel);
        assert_eq!(received.content, expected.content);
    }
}

// ============================================================================
// Bidirectional Communication Tests
// ============================================================================

#[tokio::test]
async fn test_bidirectional_message_flow() {
    let (bus, mut in_rx, mut out_rx) = MessageBus::channels();

    // Publish inbound and outbound messages
    let inbound = InboundMessage::new("telegram", "user", "chat", "Question");
    let outbound = OutboundMessage::new("telegram", "chat", "Answer");

    bus.publish_inbound(inbound)
        .expect("Should publish inbound");
    bus.publish_outbound(outbound)
        .expect("Should publish outbound");

    // Receive both
    let received_inbound = in_rx.recv().await.expect("Should receive inbound");
    let received_outbound = out_rx.recv().await.expect("Should receive outbound");

    assert_eq!(received_inbound.content, "Question");
    assert_eq!(received_outbound.content, "Answer");
}

// ============================================================================
// Clone and Shared Bus Tests
// ============================================================================

#[tokio::test]
async fn test_bus_clone_shares_channels() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    let bus_clone = bus.clone();

    // Send from original
    bus.publish_inbound(InboundMessage::new("ch", "u1", "chat", "From original"))
        .expect("Should publish");

    // Send from clone
    bus_clone
        .publish_inbound(InboundMessage::new("ch", "u2", "chat", "From clone"))
        .expect("Should publish");

    // Both should be received on the same receiver
    let msg1 = in_rx.recv().await.expect("Should receive first");
    let msg2 = in_rx.recv().await.expect("Should receive second");

    let contents: Vec<String> = vec![msg1.content.clone(), msg2.content.clone()];
    assert!(contents.contains(&"From original".to_string()));
    assert!(contents.contains(&"From clone".to_string()));
}

#[tokio::test]
async fn test_multiple_clones() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    let bus2 = bus.clone();
    let bus3 = bus.clone();

    // Send from all three
    bus.publish_inbound(InboundMessage::new("ch", "u", "c", "1"))
        .unwrap();
    bus2.publish_inbound(InboundMessage::new("ch", "u", "c", "2"))
        .unwrap();
    bus3.publish_inbound(InboundMessage::new("ch", "u", "c", "3"))
        .unwrap();

    // All three should be received
    let mut contents = Vec::new();
    for _ in 0..3 {
        contents.push(in_rx.recv().await.unwrap().content);
    }

    contents.sort();
    assert_eq!(contents, vec!["1", "2", "3"]);
}

// ============================================================================
// Concurrent Publishing Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_publishing() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    let bus1 = bus.clone();
    let bus2 = bus.clone();
    let bus3 = bus.clone();

    // Spawn concurrent publishers
    let handle1 = tokio::spawn(async move {
        for i in 0..10 {
            bus1.publish_inbound(InboundMessage::new("ch", "u", "c", format!("task1-{}", i)))
                .unwrap();
        }
    });

    let handle2 = tokio::spawn(async move {
        for i in 0..10 {
            bus2.publish_inbound(InboundMessage::new("ch", "u", "c", format!("task2-{}", i)))
                .unwrap();
        }
    });

    let handle3 = tokio::spawn(async move {
        for i in 0..10 {
            bus3.publish_inbound(InboundMessage::new("ch", "u", "c", format!("task3-{}", i)))
                .unwrap();
        }
    });

    // Wait for all publishers
    let _ = tokio::join!(handle1, handle2, handle3);

    // Receive all 30 messages
    let mut count = 0;
    while let Ok(Some(_)) =
        tokio::time::timeout(std::time::Duration::from_millis(100), in_rx.recv()).await
    {
        count += 1;
    }

    assert_eq!(count, 30);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_publish_error_when_receiver_dropped() {
    let (bus, in_rx, out_rx) = MessageBus::channels();

    // Drop the receivers
    drop(in_rx);
    drop(out_rx);

    // Publishing should fail when receivers are dropped
    let msg = InboundMessage::new("ch", "u", "c", "test");
    let result = bus.publish_inbound(msg);

    assert!(result.is_err());
}

#[tokio::test]
async fn test_publish_outbound_error_when_receiver_dropped() {
    let (bus, in_rx, out_rx) = MessageBus::channels();

    drop(in_rx);
    drop(out_rx);

    let msg = OutboundMessage::new("ch", "c", "test");
    let result = bus.publish_outbound(msg);

    assert!(result.is_err());
}

#[tokio::test]
async fn test_partial_receiver_drop() {
    let (bus, in_rx, mut out_rx) = MessageBus::channels();

    // Drop only inbound receiver
    drop(in_rx);

    // Outbound should still work
    let outbound_result = bus.publish_outbound(OutboundMessage::new("ch", "c", "test"));
    assert!(outbound_result.is_ok());

    let received = out_rx.recv().await.expect("Should receive outbound");
    assert_eq!(received.content, "test");

    // Inbound should fail
    let inbound_result = bus.publish_inbound(InboundMessage::new("ch", "u", "c", "test"));
    assert!(inbound_result.is_err());
}

// ============================================================================
// High Volume Tests
// ============================================================================

#[tokio::test]
async fn test_high_volume_inbound() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    const COUNT: usize = 1000;

    // Send many messages
    for i in 0..COUNT {
        bus.publish_inbound(InboundMessage::new("ch", "u", "c", format!("msg-{}", i)))
            .expect("Should publish");
    }

    // Receive all messages
    let mut received = 0;
    while received < COUNT {
        let msg = in_rx.recv().await.expect("Should receive");
        assert!(msg.content.starts_with("msg-"));
        received += 1;
    }

    assert_eq!(received, COUNT);
}

#[tokio::test]
async fn test_high_volume_outbound() {
    let (bus, in_rx, mut out_rx) = MessageBus::channels();
    drop(in_rx);

    const COUNT: usize = 1000;

    for i in 0..COUNT {
        bus.publish_outbound(OutboundMessage::new("ch", "c", format!("msg-{}", i)))
            .expect("Should publish");
    }

    let mut received = 0;
    while received < COUNT {
        let _ = out_rx.recv().await.expect("Should receive");
        received += 1;
    }

    assert_eq!(received, COUNT);
}

// ============================================================================
// Integration with Message Features
// ============================================================================

#[tokio::test]
async fn test_full_message_features_through_bus() {
    let (bus, mut in_rx, out_rx) = MessageBus::channels();
    drop(out_rx);

    let msg = InboundMessage::new(
        "secure-channel",
        "agent-007",
        "mission-critical",
        "Intel received",
    )
    .with_media("/intel/photo1.jpg")
    .with_media("/intel/photo2.jpg")
    .with_metadata("priority", "high")
    .with_metadata("classification", "top-secret")
    .with_metadata("timestamp", 1234567890i64);

    let session_key_before = msg.session_key();
    bus.publish_inbound(msg).expect("Should publish");

    let received = in_rx.recv().await.expect("Should receive");

    assert_eq!(received.session_key(), session_key_before);
    assert_eq!(received.channel, "secure-channel");
    assert_eq!(received.sender_id, "agent-007");
    assert_eq!(received.chat_id, "mission-critical");
    assert_eq!(received.media.len(), 2);
    assert_eq!(received.metadata.len(), 3);
}
