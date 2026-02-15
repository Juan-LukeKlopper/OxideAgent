//! Unit tests for the container module.

use OxideAgent::config::{AgentConfig, AgentType, InterfaceType, OxideConfig as Config};
use OxideAgent::core::container::Container;
use OxideAgent::types::AppEvent;
use tokio::sync::mpsc;

#[test]
fn test_container_new() {
    let config = create_test_config();
    let container = Container::new(config);

    // Verify config is accessible
    assert_eq!(container.config().agent.name, "Qwen");
}

#[tokio::test]
async fn test_container_build_tool_registry() {
    let config = create_test_config();
    let mut container = Container::new(config);

    let tool_registry = container.build_tool_registry().await;
    assert!(tool_registry.is_ok());

    let tool_registry = tool_registry.unwrap();
    let definitions = tool_registry.definitions();

    // Should have 3 default tools: write_file, read_file, run_shell_command
    assert_eq!(definitions.len(), 3);

    let names: Vec<String> = definitions
        .iter()
        .map(|t| t.function.name.clone())
        .collect();
    assert!(names.contains(&"write_file".to_string()));
    assert!(names.contains(&"read_file".to_string()));
    assert!(names.contains(&"run_shell_command".to_string()));
}

#[test]
fn test_container_build_session_manager() {
    let config = create_test_config();
    let mut container = Container::new(config);

    let session_manager = container.build_session_manager();
    assert!(session_manager.is_ok());
}

#[tokio::test]
async fn test_container_build_orchestrator() {
    let config = create_test_config();
    let mut container = Container::new(config);

    let (tx, rx) = mpsc::channel::<AppEvent>(32);

    let orchestrator = container.build_orchestrator(tx, rx).await;
    assert!(orchestrator.is_ok());

    // We can't access private fields directly, so we just verify the orchestrator was created
}

#[test]
fn test_container_config_accessors() {
    let config = create_test_config();
    let mut container = Container::new(config);

    // Test config accessor
    assert_eq!(container.config().agent.name, "Qwen");

    // Test config mutable accessor (though we won't actually modify it in this test)
    let config_ref = container.config_mut();
    assert_eq!(config_ref.agent.name, "Qwen");
}

#[tokio::test]
async fn test_container_multiple_build_calls() {
    let config = create_test_config();
    let mut container = Container::new(config);

    // Building components multiple times should not cause issues

    let tool_registry1 = container.build_tool_registry().await;
    assert!(tool_registry1.is_ok());

    let tool_registry2 = container.build_tool_registry().await;
    assert!(tool_registry2.is_ok());
}

fn create_test_config() -> Config {
    Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(), // Updated to match default
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
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
            model: Some("qwen3:4b".to_string()),
        },
        multi_agent: OxideAgent::config::MultiAgentConfig::default(),
        web: None,
        telegram: None,
        discord: None,
    }
}
