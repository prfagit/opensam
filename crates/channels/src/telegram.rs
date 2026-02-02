//! Telegram channel implementation

use async_trait::async_trait;
use opensam_bus::{InboundMessage, MessageBus, OutboundMessage};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::{debug, error, info};

use crate::Channel;

/// Telegram channel configuration
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub token: String,
    pub allow_from: Vec<String>,
}

/// Telegram channel implementation
pub struct TelegramChannel {
    config: TelegramConfig,
    bus: MessageBus,
}

impl TelegramChannel {
    /// Create a new Telegram channel
    pub fn new(config: TelegramConfig, bus: MessageBus) -> Self {
        Self { config, bus }
    }

    /// Convert markdown to Telegram HTML
    ///
    /// Process:
    /// 1. First escape HTML special characters (&, <, >)
    /// 2. Then convert markdown patterns to HTML with proper closing tags
    fn markdown_to_html(text: &str) -> String {
        // Step 1: Escape HTML special characters
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        // Step 2: Convert markdown to HTML
        Self::convert_markdown(&escaped)
    }

    /// Convert markdown patterns to HTML after escaping
    ///
    /// Processing order matters:
    /// - Code blocks (```) must be processed before inline code (`)
    /// - Bold (**) must be processed before italic (*)
    fn convert_markdown(text: &str) -> String {
        let mut result = String::with_capacity(text.len() * 2);
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '`' => {
                    // Check for code block (```)
                    if chars.peek() == Some(&'`') {
                        chars.next();
                        if chars.peek() == Some(&'`') {
                            chars.next();
                            // Found opening ```
                            Self::process_code_block(&mut chars, &mut result);
                        } else {
                            // Only two backticks, treat as inline code
                            result.push_str("<code>`</code>");
                        }
                    } else {
                        // Inline code
                        Self::process_inline_code(&mut chars, &mut result);
                    }
                }
                '*' => {
                    // Check for bold (**)
                    if chars.peek() == Some(&'*') {
                        chars.next();
                        Self::process_bold(&mut chars, &mut result, "**");
                    } else {
                        // Italic
                        Self::process_italic(&mut chars, &mut result, "*");
                    }
                }
                '_' => {
                    // Check for bold (__)
                    if chars.peek() == Some(&'_') {
                        chars.next();
                        Self::process_bold(&mut chars, &mut result, "__");
                    } else {
                        // Italic
                        Self::process_italic(&mut chars, &mut result, "_");
                    }
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        result
    }

    fn process_code_block(chars: &mut std::iter::Peekable<std::str::Chars>, result: &mut String) {
        let mut content = String::new();
        let mut backtick_count = 0;
        let mut found_closing = false;

        for ch in chars.by_ref() {
            if ch == '`' {
                backtick_count += 1;
                if backtick_count == 3 {
                    // Found closing ```
                    found_closing = true;
                    break;
                }
            } else {
                // Add any accumulated backticks to content
                for _ in 0..backtick_count {
                    content.push('`');
                }
                backtick_count = 0;
                content.push(ch);
            }
        }

        // Add remaining backticks only if we didn't find a proper closing ```
        // (i.e., we reached end of input)
        if !found_closing {
            for _ in 0..backtick_count {
                content.push('`');
            }
        }

        result.push_str("<pre>");
        result.push_str(&content);
        result.push_str("</pre>");
    }

    fn process_inline_code(chars: &mut std::iter::Peekable<std::str::Chars>, result: &mut String) {
        let mut content = String::new();

        for ch in chars.by_ref() {
            if ch == '`' {
                break;
            }
            content.push(ch);
        }

        result.push_str("<code>");
        result.push_str(&content);
        result.push_str("</code>");
    }

    fn process_bold(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        result: &mut String,
        closer: &str,
    ) {
        let mut content = String::new();
        let first_char = closer.chars().next().unwrap();

        while let Some(ch) = chars.next() {
            if ch == first_char && chars.peek() == Some(&first_char) {
                chars.next();
                break;
            }
            // Handle nested italic within bold
            if (ch == '*' || ch == '_') && chars.peek() != Some(&first_char) {
                content.push_str("<i>");
                let close_char = ch;
                while let Some(inner_ch) = chars.next() {
                    if inner_ch == close_char {
                        break;
                    }
                    // Check for nested bold closing while in italic
                    if inner_ch == first_char && chars.peek() == Some(&first_char) {
                        // Put back the characters and exit italic
                        content.push(inner_ch);
                        break;
                    }
                    content.push(inner_ch);
                }
                content.push_str("</i>");
            } else {
                content.push(ch);
            }
        }

        result.push_str("<b>");
        result.push_str(&content);
        result.push_str("</b>");
    }

    fn process_italic(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        result: &mut String,
        closer: &str,
    ) {
        let mut content = String::new();
        let close_char = closer.chars().next().unwrap();

        for ch in chars.by_ref() {
            if ch == close_char {
                break;
            }
            content.push(ch);
        }

        result.push_str("<i>");
        result.push_str(&content);
        result.push_str("</i>");
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled || self.config.token.is_empty() {
            return Ok(());
        }

        info!("Starting Telegram channel");

        let bot = Bot::new(&self.config.token);
        let bus = self.bus.clone();
        let allow_from = self.config.allow_from.clone();

        teloxide::repl(bot, move |msg: Message, _bot: Bot| {
            let bus = bus.clone();
            let allow_from = allow_from.clone();

            async move {
                if let Some(text) = msg.text() {
                    let user = msg.from();
                    let chat_id = msg.chat.id;

                    // Check if allowed
                    let sender_id = user.map(|u| u.id.to_string()).unwrap_or_default();
                    if !allow_from.is_empty() && !allow_from.contains(&sender_id) {
                        debug!("Ignoring message from unauthorized user: {}", sender_id);
                        return Ok(());
                    }

                    let inbound = InboundMessage::new(
                        "telegram",
                        sender_id,
                        chat_id.to_string(),
                        text.to_string(),
                    );

                    if let Err(e) = bus.publish_inbound(inbound) {
                        error!("Failed to publish message: {}", e);
                    }
                }
                Ok(())
            }
        })
        .await;

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping Telegram channel");
        Ok(())
    }

    async fn send(
        &self,
        msg: &OutboundMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bot = Bot::new(&self.config.token);
        let chat_id: i64 = msg.chat_id.parse()?;
        let html_content = Self::markdown_to_html(&msg.content);

        bot.send_message(ChatId(chat_id), html_content)
            .parse_mode(ParseMode::Html)
            .await?;

        Ok(())
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        if self.config.allow_from.is_empty() {
            return true;
        }
        self.config.allow_from.contains(&sender_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create a mock MessageBus for testing
    fn create_mock_bus() -> MessageBus {
        let (in_tx, _in_rx) = tokio::sync::mpsc::unbounded_channel();
        let (out_tx, _out_rx) = tokio::sync::mpsc::unbounded_channel();
        MessageBus::new(in_tx, out_tx)
    }

    // =========================================================================
    // TelegramConfig Tests
    // =========================================================================

    #[test]
    fn test_telegram_config_creation_enabled() {
        let config = TelegramConfig {
            enabled: true,
            token: "test_token_123".to_string(),
            allow_from: vec!["user1".to_string(), "user2".to_string()],
        };

        assert!(config.enabled);
        assert_eq!(config.token, "test_token_123");
        assert_eq!(config.allow_from.len(), 2);
        assert!(config.allow_from.contains(&"user1".to_string()));
        assert!(config.allow_from.contains(&"user2".to_string()));
    }

    #[test]
    fn test_telegram_config_creation_disabled() {
        let config = TelegramConfig {
            enabled: false,
            token: "".to_string(),
            allow_from: vec![],
        };

        assert!(!config.enabled);
        assert!(config.token.is_empty());
        assert!(config.allow_from.is_empty());
    }

    #[test]
    fn test_telegram_config_clone() {
        let config = TelegramConfig {
            enabled: true,
            token: "secret_token".to_string(),
            allow_from: vec!["user1".to_string()],
        };

        let cloned = config.clone();
        assert_eq!(cloned.enabled, config.enabled);
        assert_eq!(cloned.token, config.token);
        assert_eq!(cloned.allow_from, config.allow_from);
    }

    // =========================================================================
    // TelegramChannel Creation Tests
    // =========================================================================

    #[test]
    fn test_telegram_channel_new() {
        let config = TelegramConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec![],
        };
        let bus = create_mock_bus();

        let channel = TelegramChannel::new(config.clone(), bus);

        assert_eq!(channel.name(), "telegram");
        assert!(channel.config.enabled);
        assert_eq!(channel.config.token, "test_token");
    }

    #[test]
    fn test_telegram_channel_name() {
        let config = TelegramConfig {
            enabled: false,
            token: "".to_string(),
            allow_from: vec![],
        };
        let bus = create_mock_bus();

        let channel = TelegramChannel::new(config, bus);

        assert_eq!(channel.name(), "telegram");
    }

    // =========================================================================
    // markdown_to_html Tests
    // =========================================================================

    #[test]
    fn test_markdown_to_html_bold_double_asterisk() {
        let input = "This is **bold** text";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "This is <b>bold</b> text");
    }

    #[test]
    fn test_markdown_to_html_bold_double_underscore() {
        let input = "This is __bold__ text";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "This is <b>bold</b> text");
    }

    #[test]
    fn test_markdown_to_html_italic_single_asterisk() {
        let input = "This is *italic* text";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "This is <i>italic</i> text");
    }

    #[test]
    fn test_markdown_to_html_italic_single_underscore() {
        let input = "This is _italic_ text";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "This is <i>italic</i> text");
    }

    #[test]
    fn test_markdown_to_html_code_inline() {
        let input = "Use `code` here";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "Use <code>code</code> here");
    }

    #[test]
    fn test_markdown_to_html_preformatted() {
        let input = "Code block: ```code``` end";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "Code block: <pre>code</pre> end");
    }

    #[test]
    fn test_markdown_to_html_combined_formatting() {
        let input = "**Bold** and *italic* and `code`";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(
            result,
            "<b>Bold</b> and <i>italic</i> and <code>code</code>"
        );
    }

    #[test]
    fn test_markdown_to_html_plain_text() {
        let input = "Just plain text without formatting";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "Just plain text without formatting");
    }

    #[test]
    fn test_markdown_to_html_empty_string() {
        let input = "";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_markdown_to_html_multiple_bold() {
        let input = "**First** and **Second** bold";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "<b>First</b> and <b>Second</b> bold");
    }

    #[test]
    fn test_markdown_to_html_multiple_code() {
        let input = "`first` and `second` code";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "<code>first</code> and <code>second</code> code");
    }

    #[test]
    fn test_markdown_to_html_html_escaping() {
        let input = "Use <script>alert('xss')</script> and & more";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(
            result,
            "Use &lt;script&gt;alert('xss')&lt;/script&gt; and &amp; more"
        );
    }

    #[test]
    fn test_markdown_to_html_code_block_with_newlines() {
        let input = "```line1\nline2\nline3```";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "<pre>line1\nline2\nline3</pre>");
    }

    #[test]
    fn test_markdown_to_html_nested_formatting() {
        // Bold with italic inside: **bold *and italic***
        let input = "**bold *and italic***";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "<b>bold <i>and italic</i></b>");
    }

    #[test]
    fn test_markdown_to_html_code_with_html_chars() {
        let input = "`x < y && y > z`";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "<code>x &lt; y &amp;&amp; y &gt; z</code>");
    }

    #[test]
    fn test_markdown_to_html_bold_in_code() {
        // Code blocks should preserve literal ** and *
        let input = "```**not bold**```";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "<pre>**not bold**</pre>");
    }

    #[test]
    fn test_markdown_to_html_mixed_bold_styles() {
        let input = "**asterisk bold** and __underscore bold__";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(result, "<b>asterisk bold</b> and <b>underscore bold</b>");
    }

    #[test]
    fn test_markdown_to_html_mixed_italic_styles() {
        let input = "*asterisk italic* and _underscore italic_";
        let result = TelegramChannel::markdown_to_html(input);
        assert_eq!(
            result,
            "<i>asterisk italic</i> and <i>underscore italic</i>"
        );
    }

    // =========================================================================
    // is_allowed Tests
    // =========================================================================

    #[test]
    fn test_is_allowed_empty_allow_from_allows_all() {
        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            allow_from: vec![], // Empty means allow all
        };
        let bus = create_mock_bus();
        let channel = TelegramChannel::new(config, bus);

        assert!(channel.is_allowed("any_user"));
        assert!(channel.is_allowed("123456"));
        assert!(channel.is_allowed(""));
        assert!(channel.is_allowed("@username"));
    }

    #[test]
    fn test_is_allowed_specific_user_id_allowed() {
        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            allow_from: vec!["user123".to_string(), "user456".to_string()],
        };
        let bus = create_mock_bus();
        let channel = TelegramChannel::new(config, bus);

        assert!(channel.is_allowed("user123"));
        assert!(channel.is_allowed("user456"));
    }

    #[test]
    fn test_is_allowed_specific_user_id_denied() {
        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            allow_from: vec!["user123".to_string(), "user456".to_string()],
        };
        let bus = create_mock_bus();
        let channel = TelegramChannel::new(config, bus);

        assert!(!channel.is_allowed("user789"));
        assert!(!channel.is_allowed("user"));
        assert!(!channel.is_allowed("USER123")); // Case sensitive
    }

    #[test]
    fn test_is_allowed_single_user_in_list() {
        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            allow_from: vec!["admin".to_string()],
        };
        let bus = create_mock_bus();
        let channel = TelegramChannel::new(config, bus);

        assert!(channel.is_allowed("admin"));
        assert!(!channel.is_allowed("user"));
        assert!(!channel.is_allowed("Admin"));
    }

    #[test]
    fn test_is_allowed_numeric_user_ids() {
        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            allow_from: vec!["123456789".to_string(), "987654321".to_string()],
        };
        let bus = create_mock_bus();
        let channel = TelegramChannel::new(config, bus);

        assert!(channel.is_allowed("123456789"));
        assert!(channel.is_allowed("987654321"));
        assert!(!channel.is_allowed("12345678"));
        assert!(!channel.is_allowed("1234567890"));
    }

    // =========================================================================
    // Async Method Tests (basic smoke tests)
    // =========================================================================

    #[tokio::test]
    async fn test_telegram_channel_stop() {
        let config = TelegramConfig {
            enabled: true,
            token: "token".to_string(),
            allow_from: vec![],
        };
        let bus = create_mock_bus();
        let mut channel = TelegramChannel::new(config, bus);

        let result = channel.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_telegram_channel_start_disabled() {
        let config = TelegramConfig {
            enabled: false,
            token: "token".to_string(),
            allow_from: vec![],
        };
        let bus = create_mock_bus();
        let mut channel = TelegramChannel::new(config, bus);

        // Should return Ok immediately when disabled
        let result = channel.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_telegram_channel_start_empty_token() {
        let config = TelegramConfig {
            enabled: true,
            token: "".to_string(),
            allow_from: vec![],
        };
        let bus = create_mock_bus();
        let mut channel = TelegramChannel::new(config, bus);

        // Should return Ok immediately when token is empty
        let result = channel.start().await;
        assert!(result.is_ok());
    }
}
