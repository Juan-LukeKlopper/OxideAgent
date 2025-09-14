//! Configuration management for the OxideAgent system.
//!
//! This module handles configuration parsing from command line arguments,
//! environment variables, and configuration files.

use clap::Parser;
use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The agent type to use
    pub agent: AgentConfig,

    /// Whether to disable streaming
    pub no_stream: bool,

    /// Session name (if any)
    pub session: Option<String>,

    /// Whether to list sessions
    pub list_sessions: bool,

    /// MCP server URL (if any)
    pub mcp_server: Option<String>,

    /// MCP authentication token (if any)
    pub mcp_auth_token: Option<String>,

    /// Interface type to use
    pub interface: InterfaceType,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// The agent type
    pub agent_type: AgentType,

    /// The model name
    pub model: String,

    /// The agent name
    pub name: String,

    /// The system prompt
    pub system_prompt: String,
}

/// Agent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    Qwen,
    Llama,
    Granite,
}

/// Interface types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterfaceType {
    Tui,
    // In the future we could add Web, Telegram, etc.
}

impl Config {
    /// Create a new configuration from command line arguments
    pub fn from_args() -> Self {
        let args = crate::cli::Args::parse();
        Self::from_cli_args(args)
    }

    /// Create a new configuration from parsed CLI arguments
    pub fn from_cli_args(args: crate::cli::Args) -> Self {
        let agent_config = AgentConfig {
            agent_type: match args.agent {
                crate::cli::AgentType::Qwen => AgentType::Qwen,
                crate::cli::AgentType::Llama => AgentType::Llama,
                crate::cli::AgentType::Granite => AgentType::Granite,
            },
            model: args.agent.model().to_string(),
            name: args.agent.name().to_string(),
            system_prompt: args.agent.system_prompt().to_string(),
        };

        Self {
            agent: agent_config,
            no_stream: args.no_stream,
            session: args.session,
            list_sessions: args.list_sessions,
            mcp_server: args.mcp_server,
            mcp_auth_token: args.mcp_auth_token,
            interface: match args.interface {
                crate::cli::InterfaceType::Tui => InterfaceType::Tui,
            },
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate that if an MCP server is specified, an auth token is also provided
        if self.mcp_server.is_some() && self.mcp_auth_token.is_none() {
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
