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
    // In the future we could add Web, Telegram, etc.
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

        Ok(())
    }
}
