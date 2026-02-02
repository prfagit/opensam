//! FOX-DIE: Configuration management for OpenSAM
//!
//! Handles loading and saving mission parameters from encrypted storage.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

pub mod paths;

pub use paths::{config_path, data_dir, workspace_path};

/// Errors in configuration systems
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("DATA LINK ERROR: {0}")]
    Io(#[from] std::io::Error),

    #[error("DECRYPTION FAILED: {0}")]
    Json(#[from] serde_json::Error),

    #[error("INTEL NOT FOUND: {0}")]
    NotFound(PathBuf),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// SOLITON network configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,
}

/// All SOLITON network nodes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SolitonConfig {
    #[serde(default)]
    pub anthropic: ProviderConfig,
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub openrouter: ProviderConfig,
    #[serde(default)]
    pub vllm: ProviderConfig,
}

/// WhatsApp frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_bridge_url")]
    pub bridge_url: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bridge_url: default_bridge_url(),
            allow_from: Vec::new(),
        }
    }
}

fn default_bridge_url() -> String {
    "ws://localhost:3001".to_string()
}

/// Telegram frequency
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

/// All frequency configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrequencyConfig {
    #[serde(default)]
    pub whatsapp: WhatsAppConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
}

/// Default operative parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperativeDefaults {
    #[serde(default = "default_workspace")]
    pub workspace: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_iterations")]
    pub max_tool_iterations: u32,
    #[serde(default = "default_session_max_messages")]
    pub session_max_messages: usize,
}

impl Default for OperativeDefaults {
    fn default() -> Self {
        Self {
            workspace: default_workspace(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            max_tool_iterations: default_max_iterations(),
            session_max_messages: default_session_max_messages(),
        }
    }
}

fn default_workspace() -> String {
    "~/.opensam/ops".to_string()
}

fn default_model() -> String {
    "anthropic/claude-sonnet-4".to_string()
}

fn default_max_tokens() -> u32 {
    8192
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_iterations() -> u32 {
    20
}

fn default_session_max_messages() -> usize {
    100
}

/// Operative configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperativeConfig {
    #[serde(default)]
    pub defaults: OperativeDefaults,
}

/// Web search TOOLKIT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_max_results() -> u32 {
    5
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            max_results: default_max_results(),
        }
    }
}

/// Web TOOLKIT configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebToolkitConfig {
    #[serde(default)]
    pub search: WebSearchConfig,
}

/// TOOLKIT configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolkitConfig {
    #[serde(default)]
    pub web: WebToolkitConfig,
}

/// Gateway deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    18789
}

/// Root mission parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub operative: OperativeConfig,
    #[serde(default)]
    pub frequency: FrequencyConfig,
    #[serde(default, rename = "soliton")]
    pub providers: SolitonConfig,
    #[serde(default)]
    pub deploy: DeployConfig,
    #[serde(default)]
    pub toolkit: ToolkitConfig,
}

impl Config {
    /// Load mission parameters from secure storage
    pub async fn load() -> Result<Self> {
        let path = config_path();
        Self::load_from(&path).await
    }

    /// Load from specific location
    pub async fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            info!("◆ NO INTEL FOUND AT {:?}, USING DEFAULTS", path);
            return Ok(Config::default());
        }

        debug!("◆ DECRYPTING INTEL FROM {:?}", path);
        let content = tokio::fs::read_to_string(path).await?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save mission parameters
    pub async fn save(&self) -> Result<()> {
        let path = config_path();
        self.save_to(&path).await
    }

    /// Save to specific location
    pub async fn save_to(&self, path: &Path) -> Result<()> {
        debug!("◆ ENCRYPTING INTEL TO {:?}", path);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Get operations theater path
    pub fn workspace_path(&self) -> PathBuf {
        let path = &self.operative.defaults.workspace;
        if let Some(rest) = path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest);
            }
        } else if path == "~" {
            if let Some(home) = dirs::home_dir() {
                return home;
            }
        }
        PathBuf::from(path)
    }

    /// Get SOLITON access key
    pub fn api_key(&self) -> Option<String> {
        let key = self.providers.openrouter.api_key.clone();
        if !key.is_empty() {
            return Some(key);
        }

        let key = self.providers.anthropic.api_key.clone();
        if !key.is_empty() {
            return Some(key);
        }

        let key = self.providers.openai.api_key.clone();
        if !key.is_empty() {
            return Some(key);
        }

        let key = self.providers.vllm.api_key.clone();
        if !key.is_empty() {
            return Some(key);
        }

        None
    }

    /// Get SOLITON frequency
    pub fn api_base(&self) -> Option<String> {
        if !self.providers.openrouter.api_key.is_empty() {
            return self
                .providers
                .openrouter
                .api_base
                .clone()
                .or_else(|| Some("https://openrouter.ai/api/v1".to_string()));
        }

        if let Some(ref api_base) = self.providers.vllm.api_base {
            if !api_base.is_empty() {
                return Some(api_base.clone());
            }
        }

        None
    }

    /// Verify SOLITON access
    pub fn has_api_key(&self) -> bool {
        self.api_key().is_some()
    }

    /// Get default operative model
    pub fn default_model(&self) -> String {
        self.operative.defaults.model.clone()
    }

    /// Get web intel API key
    pub fn brave_api_key(&self) -> Option<String> {
        let key = &self.toolkit.web.search.api_key;
        if key.is_empty() {
            None
        } else {
            Some(key.clone())
        }
    }

    /// Get session max messages
    pub fn session_max_messages(&self) -> usize {
        self.operative.defaults.session_max_messages
    }

    /// Get web search max results from toolkit config
    pub fn web_search_max_results(&self) -> u32 {
        self.toolkit.web.search.max_results
    }
}

/// Initialize base and secure workspace
pub async fn init() -> Result<Config> {
    let config_path = config_path();

    if config_path.exists() {
        warn!("◆ BASE ALREADY ESTABLISHED AT {:?}", config_path);
    } else {
        let config = Config::default();
        config.save().await?;
        info!("◆ FOX-DIE CONFIG ESTABLISHED AT {:?}", config_path);
    }

    let workspace = workspace_path();
    tokio::fs::create_dir_all(&workspace).await?;
    info!("◆ OPS THEATER READY AT {:?}", workspace);

    Config::load().await
}
