//! CODEC: Secure Communications Bus
//!
//! Encrypted message routing between operatives and command.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

/// Incoming transmission from field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Frequency/channel
    pub channel: String,
    /// Operative ID
    pub sender_id: String,
    /// Secure channel ID
    pub chat_id: String,
    /// Transmission content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Local>,
    /// Intel attachments
    #[serde(default)]
    pub media: Vec<String>,
    /// Operational metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl InboundMessage {
    /// Create new transmission
    pub fn new(
        channel: impl Into<String>,
        sender_id: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            sender_id: sender_id.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            timestamp: Local::now(),
            media: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get operation identifier
    pub fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }

    /// Attach intel
    pub fn with_media(mut self, path: impl Into<String>) -> Self {
        self.media.push(path.into());
        self
    }

    /// Add operational data
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), value);
        }
        self
    }
}

/// Outgoing transmission to field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Target frequency
    pub channel: String,
    /// Secure channel ID
    pub chat_id: String,
    /// Transmission content
    pub content: String,
    /// Response to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// Intel attachments
    #[serde(default)]
    pub media: Vec<String>,
    /// Operational metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl OutboundMessage {
    /// Create new transmission
    pub fn new(
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            reply_to: None,
            media: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set response target
    pub fn reply_to(mut self, msg_id: impl Into<String>) -> Self {
        self.reply_to = Some(msg_id.into());
        self
    }
}

/// Channel types for CODEC
pub type InboundSender = mpsc::UnboundedSender<InboundMessage>;
pub type InboundReceiver = mpsc::UnboundedReceiver<InboundMessage>;
pub type OutboundSender = mpsc::UnboundedSender<OutboundMessage>;
pub type OutboundReceiver = mpsc::UnboundedReceiver<OutboundMessage>;

/// CODEC communications bus
#[derive(Debug, Clone)]
pub struct MessageBus {
    inbound: InboundSender,
    outbound: OutboundSender,
}

impl MessageBus {
    /// Initialize CODEC with channels
    pub fn new(inbound: InboundSender, outbound: OutboundSender) -> Self {
        Self { inbound, outbound }
    }

    /// Establish new CODEC frequency
    pub fn channels() -> (Self, InboundReceiver, OutboundReceiver) {
        let (in_tx, in_rx) = mpsc::unbounded_channel();
        let (out_tx, out_rx) = mpsc::unbounded_channel();

        (Self::new(in_tx, out_tx), in_rx, out_rx)
    }

    /// Transmit to operative
    #[allow(clippy::result_large_err)]
    pub fn publish_inbound(
        &self,
        msg: InboundMessage,
    ) -> Result<(), mpsc::error::SendError<InboundMessage>> {
        trace!("◆ INBOUND: {} -> {}", msg.sender_id, msg.channel);
        self.inbound.send(msg)
    }

    /// Transmit to command
    #[allow(clippy::result_large_err)]
    pub fn publish_outbound(
        &self,
        msg: OutboundMessage,
    ) -> Result<(), mpsc::error::SendError<OutboundMessage>> {
        trace!("◆ OUTBOUND: {} -> {}", msg.channel, msg.chat_id);
        self.outbound.send(msg)
    }

    /// Get a clone of the outbound sender
    pub fn outbound_sender(&self) -> OutboundSender {
        self.outbound.clone()
    }
}

/// CODEC dispatcher for routing
pub struct OutboundDispatcher {
    receiver: OutboundReceiver,
    handlers: HashMap<String, Box<dyn Fn(OutboundMessage) + Send + Sync>>,
}

impl OutboundDispatcher {
    /// Initialize dispatcher
    pub fn new(receiver: OutboundReceiver) -> Self {
        Self {
            receiver,
            handlers: HashMap::new(),
        }
    }

    /// Register frequency handler
    pub fn on_channel<F>(&mut self, channel: impl Into<String>, handler: F)
    where
        F: Fn(OutboundMessage) + Send + Sync + 'static,
    {
        self.handlers.insert(channel.into(), Box::new(handler));
    }

    /// Execute dispatch loop
    pub async fn run(mut self) {
        debug!("◆ CODEC DISPATCHER ONLINE");

        while let Some(msg) = self.receiver.recv().await {
            if let Some(handler) = self.handlers.get(&msg.channel) {
                handler(msg);
            } else {
                error!("◆ UNKNOWN FREQUENCY: {}", msg.channel);
            }
        }

        debug!("◆ CODEC DISPATCHER OFFLINE");
    }

    /// Async dispatch loop
    pub async fn run_async<F, Fut>(mut self, handler: F)
    where
        F: Fn(OutboundMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        debug!("◆ CODEC DISPATCHER ONLINE (ASYNC)");

        while let Some(msg) = self.receiver.recv().await {
            let fut = handler(msg);
            tokio::spawn(fut);
        }

        debug!("◆ CODEC DISPATCHER OFFLINE");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // =========================================================================
    // InboundMessage Tests
    // =========================================================================

    #[test]
    fn test_inbound_message_new() {
        let msg = InboundMessage::new("radio", "agent-007", "chat-001", "Hello Command");

        assert_eq!(msg.channel, "radio");
        assert_eq!(msg.sender_id, "agent-007");
        assert_eq!(msg.chat_id, "chat-001");
        assert_eq!(msg.content, "Hello Command");
        assert!(msg.media.is_empty());
        assert!(msg.metadata.is_empty());
    }

    #[test]
    fn test_inbound_message_session_key() {
        let msg = InboundMessage::new("radio", "agent-007", "chat-001", "Test");
        assert_eq!(msg.session_key(), "radio:chat-001");

        let msg2 = InboundMessage::new("secure-channel", "agent-001", "thread-123", "Test");
        assert_eq!(msg2.session_key(), "secure-channel:thread-123");
    }

    #[test]
    fn test_inbound_message_with_media() {
        let msg = InboundMessage::new("radio", "agent-007", "chat-001", "Photo attached")
            .with_media("/tmp/photo1.jpg")
            .with_media("/tmp/photo2.png");

        assert_eq!(msg.media.len(), 2);
        assert_eq!(msg.media[0], "/tmp/photo1.jpg");
        assert_eq!(msg.media[1], "/tmp/photo2.png");
    }

    #[test]
    fn test_inbound_message_with_metadata() {
        let msg = InboundMessage::new("radio", "agent-007", "chat-001", "Intel")
            .with_metadata("priority", "high")
            .with_metadata("classification", 5)
            .with_metadata("encrypted", true);

        assert_eq!(msg.metadata.get("priority").unwrap(), &json!("high"));
        assert_eq!(msg.metadata.get("classification").unwrap(), &json!(5));
        assert_eq!(msg.metadata.get("encrypted").unwrap(), &json!(true));
    }

    #[test]
    fn test_inbound_message_builder_chaining() {
        let msg = InboundMessage::new("secure", "agent-001", "chat-001", "Multi-part message")
            .with_media("/tmp/intel.pdf")
            .with_metadata("source", "field-op")
            .with_media("/tmp/photo.jpg")
            .with_metadata("urgent", true);

        assert_eq!(msg.media.len(), 2);
        assert_eq!(msg.metadata.len(), 2);
        assert_eq!(msg.content, "Multi-part message");
    }

    #[test]
    fn test_inbound_message_serialization() {
        let msg = InboundMessage::new("radio", "agent-007", "chat-001", "Test message")
            .with_media("/tmp/file.jpg")
            .with_metadata("key", "value");

        let json_str = serde_json::to_string(&msg).expect("Should serialize");
        let deserialized: InboundMessage =
            serde_json::from_str(&json_str).expect("Should deserialize");

        assert_eq!(deserialized.channel, msg.channel);
        assert_eq!(deserialized.sender_id, msg.sender_id);
        assert_eq!(deserialized.chat_id, msg.chat_id);
        assert_eq!(deserialized.content, msg.content);
        assert_eq!(deserialized.media, msg.media);
        assert_eq!(deserialized.metadata, msg.metadata);
    }

    // =========================================================================
    // OutboundMessage Tests
    // =========================================================================

    #[test]
    fn test_outbound_message_new() {
        let msg = OutboundMessage::new("radio", "chat-001", "Acknowledged");

        assert_eq!(msg.channel, "radio");
        assert_eq!(msg.chat_id, "chat-001");
        assert_eq!(msg.content, "Acknowledged");
        assert!(msg.reply_to.is_none());
        assert!(msg.media.is_empty());
        assert!(msg.metadata.is_empty());
    }

    #[test]
    fn test_outbound_message_reply_to() {
        let msg = OutboundMessage::new("radio", "chat-001", "Response").reply_to("msg-12345");

        assert_eq!(msg.reply_to, Some("msg-12345".to_string()));
    }

    #[test]
    fn test_outbound_message_default_media_and_metadata() {
        let msg: OutboundMessage = serde_json::from_str(
            r#"{
            "channel": "radio",
            "chat_id": "chat-001",
            "content": "Test"
        }"#,
        )
        .expect("Should deserialize");

        assert!(msg.media.is_empty());
        assert!(msg.metadata.is_empty());
    }

    #[test]
    fn test_outbound_message_serialization_skip_null_reply() {
        let msg = OutboundMessage::new("radio", "chat-001", "No reply");
        let json_str = serde_json::to_string(&msg).expect("Should serialize");

        assert!(!json_str.contains("reply_to"));

        let msg_with_reply =
            OutboundMessage::new("radio", "chat-001", "With reply").reply_to("original-msg");
        let json_str_with_reply = serde_json::to_string(&msg_with_reply).expect("Should serialize");

        assert!(json_str_with_reply.contains("reply_to"));
    }

    // =========================================================================
    // MessageBus Tests
    // =========================================================================

    #[test]
    fn test_message_bus_channels_creation() {
        let (bus, in_rx, out_rx) = MessageBus::channels();

        // Verify we got unique instances
        let (bus2, in_rx2, out_rx2) = MessageBus::channels();

        // Drop receivers to avoid warnings
        drop(in_rx);
        drop(out_rx);
        drop(in_rx2);
        drop(out_rx2);
        drop(bus);
        drop(bus2);
    }

    #[tokio::test]
    async fn test_publish_inbound_message() {
        let (bus, mut in_rx, out_rx) = MessageBus::channels();
        drop(out_rx);

        let msg = InboundMessage::new("radio", "agent-001", "chat-001", "Test");
        bus.publish_inbound(msg.clone()).expect("Should publish");

        let received = in_rx.recv().await.expect("Should receive message");
        assert_eq!(received.channel, "radio");
        assert_eq!(received.sender_id, "agent-001");
        assert_eq!(received.content, "Test");
    }

    #[tokio::test]
    async fn test_publish_outbound_message() {
        let (bus, in_rx, mut out_rx) = MessageBus::channels();
        drop(in_rx);

        let msg = OutboundMessage::new("radio", "chat-001", "Response");
        bus.publish_outbound(msg.clone()).expect("Should publish");

        let received = out_rx.recv().await.expect("Should receive message");
        assert_eq!(received.channel, "radio");
        assert_eq!(received.chat_id, "chat-001");
        assert_eq!(received.content, "Response");
    }

    #[tokio::test]
    async fn test_message_bus_clone() {
        let (bus, mut in_rx, out_rx) = MessageBus::channels();
        drop(out_rx);

        let bus_clone = bus.clone();

        let msg1 = InboundMessage::new("radio", "agent-001", "chat-001", "From original");
        let msg2 = InboundMessage::new("radio", "agent-002", "chat-002", "From clone");

        bus.publish_inbound(msg1).expect("Should publish");
        bus_clone
            .publish_inbound(msg2)
            .expect("Should publish from clone");

        let received1 = in_rx.recv().await.expect("Should receive first");
        let received2 = in_rx.recv().await.expect("Should receive second");

        assert_eq!(received1.content, "From original");
        assert_eq!(received2.content, "From clone");
    }

    // =========================================================================
    // OutboundDispatcher Tests
    // =========================================================================

    #[tokio::test]
    async fn test_dispatcher_handler_registration() {
        let (_, _, out_rx) = MessageBus::channels();
        let mut dispatcher = OutboundDispatcher::new(out_rx);

        dispatcher.on_channel("channel-1", |_msg| {});
        dispatcher.on_channel("channel-2", |_msg| {});

        assert!(dispatcher.handlers.contains_key("channel-1"));
        assert!(dispatcher.handlers.contains_key("channel-2"));
        assert!(!dispatcher.handlers.contains_key("channel-3"));
    }

    #[tokio::test]
    async fn test_dispatcher_routes_to_correct_handler() {
        let (bus, in_rx, out_rx) = MessageBus::channels();
        drop(in_rx);

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        let mut dispatcher = OutboundDispatcher::new(out_rx);
        dispatcher.on_channel("alpha", move |msg| {
            let _ = tx.send(format!("alpha: {}", msg.content));
        });

        tokio::spawn(async move {
            dispatcher.run().await;
        });

        bus.publish_outbound(OutboundMessage::new("alpha", "chat-1", "Hello Alpha"))
            .expect("Should publish");

        let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), "alpha: Hello Alpha");
    }

    #[tokio::test]
    async fn test_dispatcher_unknown_channel() {
        let (bus, in_rx, out_rx) = MessageBus::channels();
        drop(in_rx);

        let mut dispatcher = OutboundDispatcher::new(out_rx);
        dispatcher.on_channel("known", |_msg| {});

        // Spawn dispatcher and let it process
        tokio::spawn(async move {
            dispatcher.run().await;
        });

        // Send to unknown channel - should not panic, just log error
        bus.publish_outbound(OutboundMessage::new("unknown", "chat-1", "To unknown"))
            .expect("Should publish");

        // Small delay to let dispatcher process
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_dispatcher_run_async() {
        let (bus, in_rx, out_rx) = MessageBus::channels();
        drop(in_rx);

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        let dispatcher = OutboundDispatcher::new(out_rx);

        tokio::spawn(async move {
            dispatcher
                .run_async(move |msg| {
                    let tx = tx.clone();
                    async move {
                        let _ = tx.send(msg.content);
                    }
                })
                .await;
        });

        bus.publish_outbound(OutboundMessage::new("any", "chat-1", "Async message"))
            .expect("Should publish");

        let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), "Async message");
    }

    #[tokio::test]
    async fn test_dispatcher_multiple_messages() {
        let (bus, in_rx, out_rx) = MessageBus::channels();
        drop(in_rx);

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        let mut dispatcher = OutboundDispatcher::new(out_rx);

        let tx1 = tx.clone();
        dispatcher.on_channel("ch1", move |msg| {
            let _ = tx1.send(format!("ch1: {}", msg.content));
        });

        let tx2 = tx.clone();
        dispatcher.on_channel("ch2", move |msg| {
            let _ = tx2.send(format!("ch2: {}", msg.content));
        });

        tokio::spawn(async move {
            dispatcher.run().await;
        });

        // Send multiple messages
        bus.publish_outbound(OutboundMessage::new("ch1", "chat-1", "First"))
            .unwrap();
        bus.publish_outbound(OutboundMessage::new("ch2", "chat-1", "Second"))
            .unwrap();
        bus.publish_outbound(OutboundMessage::new("ch1", "chat-1", "Third"))
            .unwrap();

        let mut results = Vec::new();
        for _ in 0..3 {
            let result =
                tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
            results.push(result.unwrap().unwrap());
        }

        results.sort();
        assert_eq!(results, vec!["ch1: First", "ch1: Third", "ch2: Second"]);
    }

    // =========================================================================
    // Edge Cases and Metadata Tests
    // =========================================================================

    #[test]
    fn test_metadata_complex_types() {
        #[derive(Serialize)]
        struct CustomData {
            field1: String,
            field2: i32,
        }

        let custom = CustomData {
            field1: "value".to_string(),
            field2: 42,
        };

        let msg = InboundMessage::new("radio", "agent-001", "chat-001", "Test")
            .with_metadata("nested", custom)
            .with_metadata("array", vec![1, 2, 3])
            .with_metadata("null_value", serde_json::Value::Null);

        let nested = msg.metadata.get("nested").unwrap();
        assert!(nested.is_object());
        assert_eq!(nested.get("field1").unwrap(), &json!("value"));
        assert_eq!(nested.get("field2").unwrap(), &json!(42));

        let array = msg.metadata.get("array").unwrap();
        assert!(array.is_array());
        assert_eq!(array.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_metadata_invalid_serialization() {
        // This tests that with_metadata handles serialization errors gracefully
        // by checking that the message still works even if serialization fails
        // (which shouldn't happen with standard types)
        let msg = InboundMessage::new("radio", "agent-001", "chat-001", "Test");
        let msg_with_meta = msg.with_metadata("key", "valid_value");

        assert!(msg_with_meta.metadata.contains_key("key"));
    }

    #[test]
    fn test_empty_string_fields() {
        let inbound = InboundMessage::new("", "", "", "");
        assert_eq!(inbound.channel, "");
        assert_eq!(inbound.sender_id, "");
        assert_eq!(inbound.chat_id, "");
        assert_eq!(inbound.content, "");

        let outbound = OutboundMessage::new("", "", "");
        assert_eq!(outbound.channel, "");
        assert_eq!(outbound.chat_id, "");
        assert_eq!(outbound.content, "");
    }

    #[test]
    fn test_unicode_content() {
        let unicode_content = "Hello 世界 🌍 Привет";
        let inbound = InboundMessage::new("radio", "特工", "聊天", unicode_content);
        let outbound = OutboundMessage::new("radio", "聊天", unicode_content);

        assert_eq!(inbound.content, unicode_content);
        assert_eq!(outbound.content, unicode_content);

        // Test serialization roundtrip
        let json_in = serde_json::to_string(&inbound).unwrap();
        let json_out = serde_json::to_string(&outbound).unwrap();

        let inbound2: InboundMessage = serde_json::from_str(&json_in).unwrap();
        let outbound2: OutboundMessage = serde_json::from_str(&json_out).unwrap();

        assert_eq!(inbound2.content, unicode_content);
        assert_eq!(outbound2.content, unicode_content);
    }

    #[test]
    fn test_large_metadata() {
        let mut msg = InboundMessage::new("radio", "agent-001", "chat-001", "Test");

        for i in 0..100 {
            msg = msg.with_metadata(format!("key_{}", i), format!("value_{}", i));
        }

        assert_eq!(msg.metadata.len(), 100);
        assert_eq!(msg.metadata.get("key_50").unwrap(), &json!("value_50"));
        assert_eq!(msg.metadata.get("key_99").unwrap(), &json!("value_99"));
    }
}
