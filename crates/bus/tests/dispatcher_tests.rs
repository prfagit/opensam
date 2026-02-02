//! Integration tests for opensam-bus OutboundDispatcher
//!
//! Tests cover:
//! - Handler registration
//! - Message routing to correct handlers
//! - Async dispatch functionality
//! - Unknown channel handling
//! - Multiple handlers and concurrent dispatch

use opensam_bus::{MessageBus, OutboundDispatcher, OutboundMessage};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

// ============================================================================
// Basic Handler Registration Tests
// ============================================================================

#[test]
fn test_dispatcher_creation() {
    let (_, _, out_rx) = MessageBus::channels();
    let _dispatcher = OutboundDispatcher::new(out_rx);
}

#[test]
fn test_single_handler_registration() {
    let (_, _, out_rx) = MessageBus::channels();
    let mut dispatcher = OutboundDispatcher::new(out_rx);

    dispatcher.on_channel("alpha", |_msg| {
        println!("Handler called");
    });
}

#[test]
fn test_multiple_handler_registration() {
    let (_, _, out_rx) = MessageBus::channels();
    let mut dispatcher = OutboundDispatcher::new(out_rx);

    dispatcher.on_channel("ch1", |_msg| {});
    dispatcher.on_channel("ch2", |_msg| {});
    dispatcher.on_channel("ch3", |_msg| {});
}

#[test]
fn test_handler_overwrite() {
    let (_, _, out_rx) = MessageBus::channels();
    let mut dispatcher = OutboundDispatcher::new(out_rx);

    // Register first handler
    dispatcher.on_channel("channel", |_msg| {
        println!("First handler");
    });

    // Register second handler for same channel (should overwrite)
    dispatcher.on_channel("channel", |_msg| {
        println!("Second handler");
    });
}

// ============================================================================
// Synchronous Dispatch Tests
// ============================================================================

#[tokio::test]
async fn test_single_message_dispatch() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("target", move |msg| {
        let _ = tx.send(msg.content);
    });

    // Spawn dispatcher
    tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Send message
    bus.publish_outbound(OutboundMessage::new("target", "chat", "Hello"))
        .expect("Should publish");

    // Wait for handler to be called
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "Hello");
}

#[tokio::test]
async fn test_multiple_channels_dispatch() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);

    dispatcher.on_channel("channel-a", move |msg| {
        let _ = tx1.send(format!("A: {}", msg.content));
    });

    dispatcher.on_channel("channel-b", move |msg| {
        let _ = tx2.send(format!("B: {}", msg.content));
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Send to channel A
    bus.publish_outbound(OutboundMessage::new("channel-a", "chat", "Message to A"))
        .unwrap();

    // Send to channel B
    bus.publish_outbound(OutboundMessage::new("channel-b", "chat", "Message to B"))
        .unwrap();

    // Receive both
    let result_a = tokio::time::timeout(std::time::Duration::from_millis(100), rx1.recv()).await;

    let result_b = tokio::time::timeout(std::time::Duration::from_millis(100), rx2.recv()).await;

    assert_eq!(result_a.unwrap().unwrap(), "A: Message to A");
    assert_eq!(result_b.unwrap().unwrap(), "B: Message to B");
}

#[tokio::test]
async fn test_unknown_channel_handling() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("known", move |msg| {
        let _ = tx.send(msg.content);
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Send to unknown channel - should not panic
    bus.publish_outbound(OutboundMessage::new("unknown", "chat", "Lost message"))
        .unwrap();

    // Send to known channel
    bus.publish_outbound(OutboundMessage::new("known", "chat", "Found message"))
        .unwrap();

    // Only known channel message should arrive
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;

    assert_eq!(result.unwrap().unwrap(), "Found message");

    // No more messages should be available
    let no_more = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await;
    assert!(no_more.is_err());
}

#[tokio::test]
async fn test_message_ordering() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("ordered", move |msg| {
        let _ = tx.send(msg.content);
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Send multiple messages in order
    for i in 0..10 {
        bus.publish_outbound(OutboundMessage::new(
            "ordered",
            "chat",
            format!("msg-{}", i),
        ))
        .unwrap();
    }

    // Receive in order
    for i in 0..10 {
        let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
        assert_eq!(result.unwrap().unwrap(), format!("msg-{}", i));
    }
}

// ============================================================================
// Async Dispatch Tests
// ============================================================================

#[tokio::test]
async fn test_async_dispatch_single() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let dispatcher = OutboundDispatcher::new(out_rx);

    tokio::spawn(async move {
        dispatcher
            .run_async(move |msg| {
                let tx = tx.clone();
                async move {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    let _ = tx.send(msg.content);
                }
            })
            .await;
    });

    bus.publish_outbound(OutboundMessage::new("any", "chat", "Async message"))
        .unwrap();

    let result = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;

    assert_eq!(result.unwrap().unwrap(), "Async message");
}

#[tokio::test]
async fn test_async_dispatch_concurrent() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let dispatcher = OutboundDispatcher::new(out_rx);

    tokio::spawn(async move {
        dispatcher
            .run_async(move |_msg| {
                let counter = counter_clone.clone();
                async move {
                    // Simulate async work
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
            .await;
    });

    // Send multiple messages quickly
    for i in 0..10 {
        bus.publish_outbound(OutboundMessage::new("any", "chat", format!("msg-{}", i)))
            .unwrap();
    }

    // Wait for all to complete
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

// ============================================================================
// Complex Message Handling Tests
// ============================================================================

#[tokio::test]
async fn test_handler_receives_full_message() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<OutboundMessage>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("full-test", move |msg| {
        let _ = tx.send(msg);
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    let original = OutboundMessage::new("full-test", "chat-123", "Test content")
        .reply_to("original-msg-id")
        .clone();

    bus.publish_outbound(original).unwrap();

    let received = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(received.channel, "full-test");
    assert_eq!(received.chat_id, "chat-123");
    assert_eq!(received.content, "Test content");
    assert_eq!(received.reply_to, Some("original-msg-id".to_string()));
}

#[tokio::test]
async fn test_handler_modifies_external_state() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let state = Arc::new(AtomicUsize::new(0));
    let state_clone = state.clone();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("increment", move |_msg| {
        state_clone.fetch_add(1, Ordering::SeqCst);
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Send multiple increment messages
    for _ in 0..5 {
        bus.publish_outbound(OutboundMessage::new("increment", "chat", "inc"))
            .unwrap();
    }

    // Give time for processing
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    assert_eq!(state.load(Ordering::SeqCst), 5);
}

#[tokio::test]
async fn test_multiple_handlers_same_channel_sequential() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
    let (tx2, _rx2) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);

    // Register first handler
    dispatcher.on_channel("shared", move |msg| {
        let _ = tx1.send(format!("handler1: {}", msg.content));
    });

    // Register second handler - this overwrites the first
    dispatcher.on_channel("shared", move |msg| {
        let _ = tx2.send(format!("handler2: {}", msg.content));
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Give dispatcher time to start
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    bus.publish_outbound(OutboundMessage::new("shared", "chat", "test"))
        .unwrap();

    // Give time for message processing
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Only second handler should receive (due to overwrite)
    // When first handler is overwritten, tx1 is dropped and channel closes
    let result = tokio::time::timeout(std::time::Duration::from_millis(50), rx1.recv()).await;

    // First handler should NOT receive anything (it was overwritten)
    // When sender is dropped, recv() returns Ok(None), not a timeout error
    match result {
        Ok(None) => (), // Channel closed as expected (sender dropped)
        Ok(Some(_)) => panic!("First handler should not receive message after being overwritten"),
        Err(_) => (), // Timeout is also acceptable (message wasn't sent)
    }
}

// ============================================================================
// High Volume and Stress Tests
// ============================================================================

#[tokio::test]
async fn test_high_volume_dispatch() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("flood", move |_msg| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    const COUNT: usize = 1000;

    // Send many messages
    for i in 0..COUNT {
        bus.publish_outbound(OutboundMessage::new("flood", "chat", format!("msg-{}", i)))
            .unwrap();
    }

    // Wait for processing
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    assert_eq!(counter.load(Ordering::Relaxed), COUNT);
}

#[tokio::test]
async fn test_dispatcher_with_bus_drop() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("test", move |msg| {
        let _ = tx.send(msg.content);
    });

    // Spawn dispatcher
    let dispatch_handle = tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Send a message
    bus.publish_outbound(OutboundMessage::new("test", "chat", "Before drop"))
        .unwrap();

    // Receive the message
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
    assert_eq!(result.unwrap().unwrap(), "Before drop");

    // Drop the bus (closes senders)
    drop(bus);

    // Dispatcher should complete
    let timeout_result =
        tokio::time::timeout(std::time::Duration::from_millis(100), dispatch_handle).await;

    assert!(timeout_result.is_ok());
}

// ============================================================================
// Channel Name Edge Cases
// ============================================================================

#[tokio::test]
async fn test_unicode_channel_names() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("频道-🚀", move |msg| {
        let _ = tx.send(msg.content);
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    bus.publish_outbound(OutboundMessage::new("频道-🚀", "chat", "Unicode channel"))
        .unwrap();

    let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;

    assert_eq!(result.unwrap().unwrap(), "Unicode channel");
}

#[tokio::test]
async fn test_empty_channel_name() {
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("", move |msg| {
        let _ = tx.send(msg.content);
    });

    tokio::spawn(async move {
        dispatcher.run().await;
    });

    bus.publish_outbound(OutboundMessage::new("", "chat", "Empty channel"))
        .unwrap();

    let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;

    assert_eq!(result.unwrap().unwrap(), "Empty channel");
}

// ============================================================================
// Integration with MessageBus
// ============================================================================

#[tokio::test]
async fn test_full_pipeline_bus_to_dispatcher() {
    // Create bus and get outbound receiver
    let (bus, in_rx, out_rx) = MessageBus::channels();
    drop(in_rx);

    let (result_tx, mut result_rx) = mpsc::unbounded_channel::<String>();
    let result_tx2 = result_tx.clone();

    // Set up dispatcher
    let mut dispatcher = OutboundDispatcher::new(out_rx);
    dispatcher.on_channel("telegram", move |msg| {
        let _ = result_tx.send(format!("Telegram: {}", msg.content));
    });
    dispatcher.on_channel("discord", move |msg| {
        let _ = result_tx2.send(format!("Discord: {}", msg.content));
    });

    // Start dispatcher
    tokio::spawn(async move {
        dispatcher.run().await;
    });

    // Publish through bus
    bus.publish_outbound(OutboundMessage::new("telegram", "chat1", "Hello Telegram"))
        .unwrap();
    bus.publish_outbound(OutboundMessage::new("discord", "chat2", "Hello Discord"))
        .unwrap();
    bus.publish_outbound(OutboundMessage::new("unknown", "chat3", "Nowhere"))
        .unwrap();
    bus.publish_outbound(OutboundMessage::new(
        "telegram",
        "chat1",
        "Another Telegram",
    ))
    .unwrap();

    // Collect results
    let mut results = Vec::new();
    for _ in 0..3 {
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(100), result_rx.recv()).await;
        results.push(result.unwrap().unwrap());
    }

    results.sort();
    assert_eq!(
        results,
        vec![
            "Discord: Hello Discord",
            "Telegram: Another Telegram",
            "Telegram: Hello Telegram"
        ]
    );
}
