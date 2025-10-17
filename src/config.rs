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

fn default_model() -> String {
    "".to_string()
}

fn default_name() -> String {
    "Qwen".to_string()
}

fn default_system_prompt() -> String {
    "You are a Rust programming expert.".to_string()
}

fn default_provider() -> String {
    "ollama".to_string()
}

pub fn default_api_base() -> String {
    "http://localhost:11434".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_from_json() {
        let json_content = r#"{
            "agent": {
                "agent_type": "Qwen",
                "model": "qwen3:4b",
                "name": "Qwen",
                "system_prompt": "You are a Rust programming expert."
            },
            "mcp": {
                "server": null,
                "auth_token": null,
                "tools": []
            },
            "no_stream": false,
            "session": null,
            "list_sessions": false,
            "interface": "Tui",
            "llm": {
                "provider": "ollama",
                "api_base": null,
                "api_key": null,
                "model": "qwen3:4b"
            }
        }"#;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        // Create a new file with .json extension
        let json_path = path.with_extension("json");
        std::fs::write(&json_path, json_content).unwrap();

        let config = OxideConfig::from_file(&json_path).unwrap();

        assert_eq!(config.agent.agent_type, AgentType::Qwen);
        assert_eq!(config.agent.model, "qwen3:4b");
        assert_eq!(config.agent.name, "Qwen");
        assert_eq!(
            config.agent.system_prompt,
            "You are a Rust programming expert."
        );
        assert_eq!(config.interface, InterfaceType::Tui);
        assert_eq!(config.llm.provider, "ollama");
    }

    #[test]
    fn test_config_from_yaml() {
        let yaml_content = r#"---
agent:
  agent_type: "Llama"
  model: "llama3.2"
  name: "Llama"
  system_prompt: "You are a helpful assistant."
mcp:
  server: ~
  auth_token: ~
  tools: []
no_stream: true
session: "test-session"
list_sessions: false
interface: "Tui"
llm:
  provider: "ollama"
  api_base: ~
  api_key: ~
  model: ~
"#;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        // Create a new file with .yaml extension
        let yaml_path = path.with_extension("yaml");
        std::fs::write(&yaml_path, yaml_content).unwrap();

        let config = OxideConfig::from_file(&yaml_path).unwrap();

        assert_eq!(config.agent.agent_type, AgentType::Llama);
        assert_eq!(config.agent.model, "llama3.2");
        assert_eq!(config.agent.name, "Llama");
        assert_eq!(config.agent.system_prompt, "You are a helpful assistant.");
        assert!(config.no_stream);
        assert_eq!(config.session, Some("test-session".to_string()));
        assert_eq!(config.interface, InterfaceType::Tui);
    }

    #[test]
    fn test_config_from_toml() {
        let toml_content = r#"no_stream = true
session = "toml-session"
list_sessions = false
interface = "Tui"

[agent]
agent_type = "Granite"
model = "smolLM2"
name = "Granite"
system_prompt = "You are a helpful assistant."

[mcp]
server = "http://localhost:8080"
auth_token = "secret-token"
tools = []

[llm]
provider = "openai"
api_key = "sk-abc123"
model = "gpt-4"
"#;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        // Create a new file with .toml extension
        let toml_path = path.with_extension("toml");
        std::fs::write(&toml_path, toml_content).unwrap();

        let config = OxideConfig::from_file(&toml_path).unwrap();

        assert_eq!(config.agent.agent_type, AgentType::Granite);
        assert_eq!(config.agent.model, "smolLM2");
        assert_eq!(config.agent.name, "Granite");
        assert!(config.no_stream);
        assert_eq!(config.session, Some("toml-session".to_string()));
        assert_eq!(config.mcp.server, Some("http://localhost:8080".to_string()));
        assert_eq!(config.mcp.auth_token, Some("secret-token".to_string()));
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.llm.api_key, Some("sk-abc123".to_string()));
        assert_eq!(config.llm.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_config_from_nonexistent_file() {
        let result = OxideConfig::from_file("/nonexistent/path/config.json");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read config file")
        );
    }

    #[test]
    fn test_config_from_invalid_json() {
        let invalid_json = r#"{"invalid": json}"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_json.as_bytes()).unwrap();
        let path = temp_file.path();

        let result = OxideConfig::from_file(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_invalid_yaml() {
        let invalid_yaml = r#"invalid: - yaml: [unclosed"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_yaml.as_bytes()).unwrap();
        let path = temp_file.path();

        let result = OxideConfig::from_file(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_invalid_toml() {
        let invalid_toml = r#"invalid = toml content with [ unclosed"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_toml.as_bytes()).unwrap();
        let path = temp_file.path();

        let result = OxideConfig::from_file(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_unsupported_format() {
        let invalid_ext = r#"{"some": "json"}"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_ext.as_bytes()).unwrap();
        // Change the extension to unsupported
        let path = temp_file.path().with_extension("unsupported");
        fs::write(&path, invalid_ext).unwrap();

        let result = OxideConfig::from_file(&path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported config format")
        );
    }
}
