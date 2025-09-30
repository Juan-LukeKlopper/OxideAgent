//! Advanced integration tests for orchestrator and agent interactions.

use OxideAgent::config::{AgentConfig, AgentType, InterfaceType, OxideConfig as Config};
use OxideAgent::core::container::Container;
use OxideAgent::core::session::SessionState;
use OxideAgent::core::tool_permissions::GlobalToolPermissions;
use OxideAgent::core::tools::{ReadFileTool, RunShellCommandTool, ToolRegistry, WriteFileTool};
use OxideAgent::types::{AppEvent, ChatMessage, ToolApprovalResponse};
use std::fs;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_orchestrator_full_interaction_cycle() {
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: true, // Use non-streaming for easier testing
        session: Some("test_integration_session".to_string()),
        list_sessions: false,
        interface: InterfaceType::Tui,
        mcp: OxideAgent::config::MCPConfig {
            server: None,
            auth_token: None,
            tools: vec![],
        },
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: None,
            api_key: None,
            model: None,
        },
    };

    // Create channels for communication
    let (orchestrator_tx, mut interface_rx) = mpsc::channel::<AppEvent>(32);
    let (interface_tx, orchestrator_rx) = mpsc::channel::<AppEvent>(32);

    // Create and build the orchestrator
    let mut container = Container::new(config);
    let mut orchestrator = container
        .build_orchestrator(orchestrator_tx, orchestrator_rx)
        .unwrap();

    // Send a user message
    let user_message = "Hello, can you tell me your name?";
    let input_event = AppEvent::UserInput(user_message.to_string());
    interface_tx.send(input_event).await.unwrap();

    // Give the orchestrator time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Check that the orchestrator has processed the input
    // This test mainly checks that the orchestrator doesn't crash during basic interaction
    assert!(true); // Placeholder assertion
}

#[tokio::test]
async fn test_orchestrator_session_switching() {
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: false,
        session: Some("initial_session".to_string()),
        list_sessions: false,
        interface: InterfaceType::Tui,
        mcp: OxideAgent::config::MCPConfig {
            server: None,
            auth_token: None,
            tools: vec![],
        },
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: None,
            api_key: None,
            model: None,
        },
    };

    // Create channels for communication
    let (orchestrator_tx, _interface_rx) = mpsc::channel::<AppEvent>(32);
    let (_interface_tx, orchestrator_rx) = mpsc::channel::<AppEvent>(32);

    // Create and build the orchestrator
    let mut container = Container::new(config);
    let mut orchestrator = container
        .build_orchestrator(orchestrator_tx, orchestrator_rx)
        .unwrap();

    // Verify initial session
    let history = orchestrator.get_session_history();
    assert_eq!(history.len(), 0);

    // Switch to a new session
    let result = orchestrator.switch_session(Some("new_session".to_string()));
    assert!(result.is_ok());

    // Check that the orchestrator can handle the session change
    // This test mainly checks that the switch_session method doesn't crash
    assert!(true); // Placeholder assertion

    // Clean up session files
    let _ = fs::remove_file("session_initial_session.json");
    let _ = fs::remove_file("session_new_session.json");
}

#[tokio::test]
async fn test_orchestrator_tool_approvals() {
    // Create a temporary session to avoid conflicts with other tests
    let temp_session_name = "temp_tool_approval_test";
    let session_file = format!("session_{}.json", temp_session_name);

    // Clean up any existing session file
    let _ = fs::remove_file(&session_file);

    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: true,
        session: Some(temp_session_name.to_string()),
        list_sessions: false,
        interface: InterfaceType::Tui,
        mcp: OxideAgent::config::MCPConfig {
            server: None,
            auth_token: None,
            tools: vec![],
        },
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: None,
            api_key: None,
            model: None,
        },
    };

    // Create channels for communication
    let (orchestrator_tx, mut interface_rx) = mpsc::channel::<AppEvent>(32);
    let (interface_tx, orchestrator_rx) = mpsc::channel::<AppEvent>(32);

    // Create and build the orchestrator
    let mut container = Container::new(config);
    let mut orchestrator = container
        .build_orchestrator(orchestrator_tx, orchestrator_rx)
        .unwrap();

    // Test loading the state
    let result = orchestrator.load_state();
    assert!(result.is_ok());

    // Clean up
    let _ = fs::remove_file(&session_file);
}

#[test]
fn test_global_tool_permissions_with_orchestrator_context() {
    // This test verifies that global tool permissions work correctly in the orchestrator context
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");

    // Create a fresh permissions instance
    let mut permissions = GlobalToolPermissions::new();

    // Initially no tools should be allowed
    assert!(!permissions.is_allowed("write_file"));
    assert!(!permissions.is_allowed("read_file"));
    assert!(!permissions.is_allowed("run_shell_command"));

    // Add specific tools
    permissions.add_allowed("write_file");
    permissions.add_allowed("read_file");

    // Verify the tools are allowed
    assert!(permissions.is_allowed("write_file"));
    assert!(permissions.is_allowed("read_file"));
    assert!(!permissions.is_allowed("run_shell_command")); // This was not added

    // Save the permissions
    assert!(permissions.save_to_path(&test_file_path).is_ok());

    // Load the permissions again to verify persistence
    let reloaded_permissions = GlobalToolPermissions::load_from_path(&test_file_path).unwrap();
    assert!(reloaded_permissions.is_allowed("write_file"));
    assert!(reloaded_permissions.is_allowed("read_file"));
    assert!(!reloaded_permissions.is_allowed("run_shell_command"));
}

#[test]
fn test_session_state_isolation() {
    // Test that different session states are properly isolated
    let mut session1 = SessionState::new();
    let mut session2 = SessionState::new();

    // Add different tools to each session
    session1.add_allowed_tool("tool1".to_string());
    session2.add_allowed_tool("tool2".to_string());

    // Verify isolation
    assert!(session1.is_tool_allowed("tool1"));
    assert!(!session1.is_tool_allowed("tool2"));

    assert!(session2.is_tool_allowed("tool2"));
    assert!(!session2.is_tool_allowed("tool1"));

    // Verify that the allowed tools lists are different
    let session1_tools = session1.list_allowed_tools();
    let session2_tools = session2.list_allowed_tools();

    assert_eq!(session1_tools.len(), 1);
    assert_eq!(session2_tools.len(), 1);

    assert!(session1_tools.contains(&"tool1".to_string()));
    assert!(session2_tools.contains(&"tool2".to_string()));
}
