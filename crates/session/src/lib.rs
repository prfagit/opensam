//! Session management for conversation history

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Default maximum number of messages in a session
pub const DEFAULT_MAX_MESSAGES: usize = 100;

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session key (channel:chat_id)
    pub key: String,
    /// Messages in the session
    pub messages: Vec<Message>,
    /// Created at timestamp
    pub created_at: DateTime<Local>,
    /// Last updated timestamp
    pub updated_at: DateTime<Local>,
    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Maximum number of messages before truncation
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
}

fn default_max_messages() -> usize {
    DEFAULT_MAX_MESSAGES
}

/// A message in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: user, assistant, system
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Local>,
    /// Additional metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session with default max_messages
    pub fn new(key: impl Into<String>) -> Self {
        Self::with_max_messages(key, DEFAULT_MAX_MESSAGES)
    }

    /// Create a new session with specified max_messages
    pub fn with_max_messages(key: impl Into<String>, max_messages: usize) -> Self {
        let now = Local::now();
        Self {
            key: key.into(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            max_messages,
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, role: impl Into<String>, content: impl Into<String>) {
        self.messages.push(Message {
            role: role.into(),
            content: content.into(),
            timestamp: Local::now(),
            extra: HashMap::new(),
        });
        self.updated_at = Local::now();

        // Enforce max messages limit
        self.enforce_max_messages();
    }

    /// Enforce max_messages limit by truncating oldest messages
    fn enforce_max_messages(&mut self) {
        if self.messages.len() > self.max_messages {
            let to_remove = self.messages.len() - self.max_messages;
            self.messages.drain(0..to_remove);
            debug!(
                "Session {} truncated to {} messages",
                self.key,
                self.messages.len()
            );
        }
    }

    /// Get message history for LLM context
    pub fn get_history(&self, max_messages: usize) -> Vec<opensam_provider::Message> {
        self.messages
            .iter()
            .skip(self.messages.len().saturating_sub(max_messages))
            .map(|m| opensam_provider::Message {
                role: m.role.clone(),
                content: Some(m.content.clone()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            })
            .collect()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Local::now();
    }

    /// Get the max messages limit
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Set the max messages limit (will truncate on next add_message if needed)
    pub fn set_max_messages(&mut self, max_messages: usize) {
        self.max_messages = max_messages;
        self.enforce_max_messages();
    }
}

/// Manages conversation sessions
pub struct SessionManager {
    sessions_dir: PathBuf,
    cache: HashMap<String, Session>,
    max_messages: usize,
}

impl SessionManager {
    /// Create a new session manager with default max_messages
    pub fn new(sessions_dir: impl AsRef<Path>) -> Self {
        Self::with_max_messages(sessions_dir, DEFAULT_MAX_MESSAGES)
    }

    /// Create a new session manager with specified max_messages
    pub fn with_max_messages(sessions_dir: impl AsRef<Path>, max_messages: usize) -> Self {
        let sessions_dir = sessions_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&sessions_dir).ok();

        Self {
            sessions_dir,
            cache: HashMap::new(),
            max_messages,
        }
    }

    /// Get or create a session
    pub async fn get_or_create(&mut self, key: &str) -> &mut Session {
        if !self.cache.contains_key(key) {
            let session = self
                .load(key)
                .await
                .unwrap_or_else(|| Session::with_max_messages(key, self.max_messages));
            self.cache.insert(key.to_string(), session);
        }
        self.cache.get_mut(key).unwrap()
    }

    /// Save a session
    pub async fn save(&self, session: &Session) -> std::io::Result<()> {
        let path = self.session_path(&session.key);
        let content = serde_json::to_string_pretty(session)?;
        tokio::fs::write(path, content).await?;
        debug!("Saved session: {}", session.key);
        Ok(())
    }

    /// Load a session from disk
    async fn load(&self, key: &str) -> Option<Session> {
        let path = self.session_path(key);
        if !path.exists() {
            return None;
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(content) => {
                match serde_json::from_str::<Session>(&content) {
                    Ok(mut session) => {
                        // Update max_messages to current setting if different
                        if session.max_messages != self.max_messages {
                            session.max_messages = self.max_messages;
                            // Truncate if necessary
                            session.enforce_max_messages();
                        }
                        debug!("Loaded session: {}", key);
                        Some(session)
                    }
                    Err(e) => {
                        warn!("Failed to parse session {}: {}", key, e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read session {}: {}", key, e);
                None
            }
        }
    }

    /// Get the file path for a session
    fn session_path(&self, key: &str) -> PathBuf {
        let safe_key = key.replace([':', '/'], "_");
        self.sessions_dir.join(format!("{}.json", safe_key))
    }

    /// Delete a session
    pub async fn delete(&mut self, key: &str) -> std::io::Result<bool> {
        self.cache.remove(key);
        let path = self.session_path(key);
        if path.exists() {
            tokio::fs::remove_file(path).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all sessions
    pub async fn list(&self) -> Vec<String> {
        let mut keys = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir(&self.sessions_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(stripped) = name.strip_suffix(".json") {
                        keys.push(stripped.replace('_', ":"));
                    }
                }
            }
        }

        keys
    }

    /// Get the max messages setting
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Update max_messages for all cached sessions and future sessions
    pub fn set_max_messages(&mut self, max_messages: usize) {
        self.max_messages = max_messages;
        // Update all cached sessions
        for session in self.cache.values_mut() {
            session.set_max_messages(max_messages);
        }
    }
}
