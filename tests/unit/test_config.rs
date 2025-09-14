//! Unit tests for the configuration module.

use OxideAgent::config::{AgentConfig, AgentType, Config, InterfaceType};

#[test]
fn test_config_creation() {
    let args = OxideAgent::cli::Args {
        agent: OxideAgent::cli::AgentType::Qwen,
        no_stream: false,
        session: Some("test_session".to_string()),
        list_sessions: false,
        mcp_server: None,
        mcp_auth_token: None,
        interface: OxideAgent::cli::InterfaceType::Tui,
    };

    let config = Config::from_cli_args(args);

    assert_eq!(config.agent.agent_type, AgentType::Qwen);
    assert_eq!(config.agent.model, "qwen3:4b");
    assert_eq!(config.agent.name, "Qwen");
    assert_eq!(config.session, Some("test_session".to_string()));
    assert_eq!(config.interface, InterfaceType::Tui);
}

#[test]
fn test_config_validation_valid() {
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a helpful assistant.".to_string(),
        },
        no_stream: false,
        session: Some("valid_session".to_string()),
        list_sessions: false,
        mcp_server: Some("http://localhost:8000".to_string()),
        mcp_auth_token: Some("test_token".to_string()),
        interface: InterfaceType::Tui,
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_mcp_missing_token() {
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a helpful assistant.".to_string(),
        },
        no_stream: false,
        session: None,
        list_sessions: false,
        mcp_server: Some("http://localhost:8000".to_string()),
        mcp_auth_token: None,
        interface: InterfaceType::Tui,
    };

    assert!(config.validate().is_err());
    assert_eq!(
        config.validate().unwrap_err().to_string(),
        "MCP server specified but no auth token provided"
    );
}

#[test]
fn test_config_validation_invalid_session_name() {
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a helpful assistant.".to_string(),
        },
        no_stream: false,
        session: Some("invalid/session".to_string()),
        list_sessions: false,
        mcp_server: None,
        mcp_auth_token: None,
        interface: InterfaceType::Tui,
    };

    assert!(config.validate().is_err());
    assert_eq!(
        config.validate().unwrap_err().to_string(),
        "Session name contains invalid characters"
    );
}
