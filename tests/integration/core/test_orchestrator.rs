//! Integration tests for the orchestrator.

use OxideAgent::config::{AgentConfig, AgentType, Config, InterfaceType};
use OxideAgent::core::agents::Agent;
use OxideAgent::core::container::Container;
use OxideAgent::core::tools::{ReadFileTool, RunShellCommandTool, ToolRegistry, WriteFileTool};
use OxideAgent::types::AppEvent;
use tokio::sync::mpsc;

#[test]
fn test_orchestrator_creation() {
    let available_models = vec!["qwen:latest".to_string(), "llama3:latest".to_string(), "granite:latest".to_string()];
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
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
            model: None,
        },
    };

    let mut container = Container::new(config);

    let (orchestrator_tx, _interface_rx) = mpsc::channel::<AppEvent>(32);
    let (_interface_tx, orchestrator_rx) = mpsc::channel::<AppEvent>(32);

    let result = container.build_orchestrator(orchestrator_tx, orchestrator_rx);
    assert!(result.is_ok());
}

#[test]
fn test_tool_registry_creation() {
    let mut tool_registry = ToolRegistry::new();
    tool_registry.add_tool(Box::new(WriteFileTool));
    tool_registry.add_tool(Box::new(ReadFileTool));
    tool_registry.add_tool(Box::new(RunShellCommandTool));

    let tools = tool_registry.definitions();
    assert_eq!(tools.len(), 3);

    let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
    assert!(tool_names.contains(&"write_file".to_string()));
    assert!(tool_names.contains(&"read_file".to_string()));
    assert!(tool_names.contains(&"run_shell_command".to_string()));
}

#[test]
fn test_agent_creation() {
    let client = Box::new(OxideAgent::core::mocks::MockOllamaClient::new());
    let agent = Agent::new("TestAgent", client);

    // Check that the agent has a system message
    assert!(!agent.history.is_empty());
    let system_message = &agent.history[0];
    assert_eq!(system_message.role, "system");
    assert_eq!(system_message.content, "TestAgent");
}
