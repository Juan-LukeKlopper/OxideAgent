//! Configuration management for the OxideAgent system.
//!
//! This module handles configuration parsing from command line arguments,
//! environment variables, and configuration files.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OxideConfig {
    /// Agent configuration section
    #[serde(default)]
    pub agent: AgentConfig,

    /// Multi-agent mode configuration
    #[serde(default)]
    pub multi_agent: MultiAgentConfig,

    /// MCP (Model Context Protocol) configuration section
    #[serde(default)]
    pub mcp: MCPConfig,

    /// Whether to disable streaming
    #[serde(default)]
    pub no_stream: bool,

    /// Session name (if any)
    pub session: Option<String>,

    /// Whether to list sessions
    #[serde(default)]
    pub list_sessions: bool,

    /// Interface type to use
    #[serde(default)]
    pub interface: InterfaceType,

    /// LLM provider configuration
    #[serde(default)]
    pub llm: LLMConfig,

    /// Web interface transport configuration
    #[serde(default)]
    pub web: Option<WebInterfaceConfig>,

    /// Telegram interface transport configuration
    #[serde(default)]
    pub telegram: Option<TelegramInterfaceConfig>,

    /// Discord interface transport configuration
    #[serde(default)]
    pub discord: Option<DiscordInterfaceConfig>,
}

/// Web interface configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebInterfaceConfig {
    /// Interface bind host
    #[serde(default = "default_web_host")]
    pub host: String,

    /// Interface bind port
    #[serde(default = "default_web_port")]
    pub port: u16,

    /// Optional API authentication token
    #[serde(default)]
    pub auth_token: Option<String>,

    /// Enable CORS for cross-origin clients
    #[serde(default)]
    pub enable_cors: bool,

    /// Maximum accepted payload size in bytes
    #[serde(default = "default_web_max_payload_bytes")]
    pub max_payload_bytes: usize,
}

impl Default for WebInterfaceConfig {
    fn default() -> Self {
        Self {
            host: default_web_host(),
            port: default_web_port(),
            auth_token: None,
            enable_cors: false,
            max_payload_bytes: default_web_max_payload_bytes(),
        }
    }
}

/// Telegram interface configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramInterfaceConfig {
    /// Telegram bot token
    pub bot_token: String,

    /// Polling interval in milliseconds
    #[serde(default = "default_telegram_polling_interval_ms")]
    pub polling_interval_ms: u64,

    /// Request timeout in seconds
    #[serde(default = "default_telegram_request_timeout_secs")]
    pub request_timeout_secs: u64,
}

/// Discord interface configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscordInterfaceConfig {
    /// Discord bot token
    pub bot_token: String,

    /// Discord application id used for command registration
    pub application_id: String,

    /// Optional default guild for local command registration
    #[serde(default)]
    pub guild_id: Option<String>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    /// The agent type
    #[serde(default)]
    pub agent_type: AgentType,

    /// The model name
    #[serde(default = "default_model")]
    pub model: String,

    /// The agent name
    #[serde(default = "default_name")]
    pub name: String,

    /// The system prompt
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
}

/// MCP (Model Context Protocol) configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPConfig {
    /// MCP server URL (if any)
    #[serde(default)]
    pub server: Option<String>,

    /// MCP authentication token (if any)
    #[serde(default)]
    pub auth_token: Option<String>,

    /// MCP tools configuration
    #[serde(default)]
    pub tools: Vec<MCPToolConfig>,
}

/// MCP Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolConfig {
    /// Tool name
    pub name: String,

    /// Tool command
    pub command: String,

    /// Tool arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Whether the tool requires approval
    #[serde(default)]
    pub requires_approval: bool,
}

/// Agent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AgentType {
    #[default]
    Qwen,
    Llama,
    Granite,
}

/// Interface types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum InterfaceType {
    #[default]
    Tui,
    Web,
    Telegram,
    Discord,
}

/// Multi-agent mode configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultiAgentConfig {
    /// Whether multi-agent mode is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Max number of concurrent agents
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,

    /// Default agents to initialize
    #[serde(default)]
    pub default_agents: Vec<AgentConfig>,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LLMConfig {
    /// Provider type (ollama, openai, etc.)
    #[serde(default = "default_provider")]
    pub provider: String,

    /// API base URL (for providers like OpenAI)
    #[serde(default = "default_api_base")]
    pub api_base: String,

    /// API key (for providers like OpenAI)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Model name
    #[serde(default)]
    pub model: Option<String>,
}

impl From<crate::cli::InterfaceType> for InterfaceType {
    fn from(cli_type: crate::cli::InterfaceType) -> Self {
        match cli_type {
            crate::cli::InterfaceType::Tui => InterfaceType::Tui,
            crate::cli::InterfaceType::Web => InterfaceType::Web,
            crate::cli::InterfaceType::Telegram => InterfaceType::Telegram,
            crate::cli::InterfaceType::Discord => InterfaceType::Discord,
        }
    }
}

pub fn default_model() -> String {
    "".to_string()
}

pub fn default_name() -> String {
    "Qwen".to_string()
}

pub fn default_system_prompt() -> String {
    "You are a Rust programming expert.".to_string()
}

pub fn default_provider() -> String {
    "ollama".to_string()
}

pub fn default_api_base() -> String {
    "http://localhost:11434".to_string()
}

pub fn default_max_agents() -> usize {
    5
}

pub fn default_web_host() -> String {
    "127.0.0.1".to_string()
}

pub fn default_web_port() -> u16 {
    8080
}

pub fn default_web_max_payload_bytes() -> usize {
    1024 * 1024
}

pub fn default_telegram_polling_interval_ms() -> u64 {
    1000
}

pub fn default_telegram_request_timeout_secs() -> u64 {
    30
}

impl OxideConfig {
    /// Create a new configuration from a file path (auto-detect format by extension)
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            anyhow::anyhow!("Failed to read config file '{}': {}", path.display(), e)
        })?;

        let config = match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => serde_json::from_str(&content).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse JSON config file '{}': {}",
                    path.display(),
                    e
                )
            })?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse YAML config file '{}': {}",
                    path.display(),
                    e
                )
            })?,
            Some("toml") => toml::from_str(&content).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse TOML config file '{}': {}",
                    path.display(),
                    e
                )
            })?,
            Some(ext) => {
                return Err(anyhow::anyhow!(
                    "Unsupported config format: {} (file: {})",
                    ext,
                    path.display()
                ));
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Config file has no extension, cannot determine format: {}",
                    path.display()
                ));
            }
        };

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate that if an MCP server is specified, an auth token is also provided
        if self.mcp.server.is_some() && self.mcp.auth_token.is_none() {
            return Err(anyhow::anyhow!(
                "MCP server specified but no auth token provided"
            ));
        }

        // Validate that the session name doesn't contain invalid characters
        if let Some(session) = &self.session {
            if session.is_empty() {
                return Err(anyhow::anyhow!("Session name cannot be empty"));
            }

            // Check for invalid characters in session name
            if session.contains('/') || session.contains('\\') || session.contains(':') {
                return Err(anyhow::anyhow!("Session name contains invalid characters"));
            }
        }

        if let Some(web) = &self.web {
            if web.port == 0 {
                return Err(anyhow::anyhow!("Web interface port must be greater than 0"));
            }
            if web.max_payload_bytes == 0 {
                return Err(anyhow::anyhow!(
                    "Web interface max_payload_bytes must be greater than 0"
                ));
            }
        }

        if let Some(telegram) = &self.telegram
            && telegram.bot_token.trim().is_empty()
        {
            return Err(anyhow::anyhow!("Telegram bot_token cannot be empty"));
        }

        if let Some(discord) = &self.discord {
            if discord.bot_token.trim().is_empty() {
                return Err(anyhow::anyhow!("Discord bot_token cannot be empty"));
            }
            if discord.application_id.trim().is_empty() {
                return Err(anyhow::anyhow!("Discord application_id cannot be empty"));
            }
        }

        Ok(())
    }
}
