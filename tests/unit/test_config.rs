//! Unit tests for the configuration module.

use OxideAgent::config::{AgentConfig, AgentType, InterfaceType, OxideConfig as Config};

#[test]
fn test_config_creation() {
    let agent_config = AgentConfig {
        agent_type: AgentType::Qwen,
        model: "qwen3:4b".to_string(),
        name: "Qwen".to_string(),
        system_prompt: "You are a Rust programming expert.".to_string(),
    };

    let config = Config {
        agent: agent_config,
        no_stream: false,
        session: Some("test_session".to_string()),
        list_sessions: false,
        interface: InterfaceType::Tui,
        mcp: OxideAgent::config::MCPConfig {
            server: None,
            auth_token: None,
            tools: vec![],
        },
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: "http://localhost:11434".to_string(),
            api_key: None,
            model: None,
        },
    };

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
        mcp: OxideAgent::config::MCPConfig {
            server: Some("http://localhost:8000".to_string()),
            auth_token: Some("test_token".to_string()),
            tools: vec![],
        },
        interface: InterfaceType::Tui,
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: "http://localhost:11434".to_string(),
            api_key: None,
            model: None,
        },
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
        mcp: OxideAgent::config::MCPConfig {
            server: Some("http://localhost:8000".to_string()),
            auth_token: None,
            tools: vec![],
        },
        interface: InterfaceType::Tui,
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: "http://localhost:11434".to_string(),
            api_key: None,
            model: None,
        },
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
        mcp: OxideAgent::config::MCPConfig {
            server: None,
            auth_token: None,
            tools: vec![],
        },
        interface: InterfaceType::Tui,
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: "http://localhost:11434".to_string(),
            api_key: None,
            model: None,
        },
    };

    assert!(config.validate().is_err());
    assert_eq!(
        config.validate().unwrap_err().to_string(),
        "Session name contains invalid characters"
    );
}
