//! Chat channels (Telegram)

use async_trait::async_trait;
use opensam_bus::OutboundMessage;

pub mod telegram;

pub use telegram::TelegramChannel;

/// Trait for chat channel implementations
#[async_trait]
pub trait Channel: Send + Sync {
    /// Channel name
    fn name(&self) -> &str;

    /// Start the channel
    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Stop the channel
    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Send a message through this channel
    async fn send(
        &self,
        msg: &OutboundMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Check if a sender is allowed
    fn is_allowed(&self, sender_id: &str) -> bool;
}
