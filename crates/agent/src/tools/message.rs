//! Message tool for sending messages to channels

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::debug;

use opensam_bus::OutboundMessage;

use super::ToolTrait;

/// Tool for sending messages to chat channels
pub struct MessageTool {
    sender: mpsc::UnboundedSender<OutboundMessage>,
    context_channel: std::sync::Mutex<Option<String>>,
    context_chat_id: std::sync::Mutex<Option<String>>,
}

impl MessageTool {
    /// Create a new message tool
    pub fn new(sender: mpsc::UnboundedSender<OutboundMessage>) -> Self {
        Self {
            sender,
            context_channel: std::sync::Mutex::new(None),
            context_chat_id: std::sync::Mutex::new(None),
        }
    }

    /// Set the context for messages (channel and chat_id)
    pub fn set_context(&self, channel: String, chat_id: String) {
        *self.context_channel.lock().unwrap() = Some(channel);
        *self.context_chat_id.lock().unwrap() = Some(chat_id);
    }
}

impl Clone for MessageTool {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            context_channel: std::sync::Mutex::new(self.context_channel.lock().unwrap().clone()),
            context_chat_id: std::sync::Mutex::new(self.context_chat_id.lock().unwrap().clone()),
        }
    }
}

#[derive(Deserialize)]
struct MessageArgs {
    content: String,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    chat_id: Option<String>,
}

#[async_trait]
impl ToolTrait for MessageTool {
    fn name(&self) -> &str {
        "message"
    }
    fn description(&self) -> &str {
        "Send a message to a chat channel."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "description": "Message content" },
                "channel": { "type": "string", "description": "Target channel (defaults to current)" },
                "chat_id": { "type": "string", "description": "Target chat ID (defaults to current)" }
            },
            "required": ["content"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: MessageArgs = serde_json::from_value(args)?;

        let channel = args
            .channel
            .or_else(|| self.context_channel.lock().unwrap().clone())
            .ok_or("No channel specified")?;

        let chat_id = args
            .chat_id
            .or_else(|| self.context_chat_id.lock().unwrap().clone())
            .ok_or("No chat_id specified")?;

        debug!("Sending message to {}:{}", channel, chat_id);

        let msg = OutboundMessage::new(channel, chat_id, args.content);
        self.sender.send(msg)?;

        Ok("Message sent".to_string())
    }
}
