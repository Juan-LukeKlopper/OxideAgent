//! Unit tests for the orchestrator module using mock objects.

use OxideAgent::config::{AgentConfig, AgentType, Config, InterfaceType};
use OxideAgent::core::agents::Agent;
use OxideAgent::core::orchestrator::Orchestrator;
use OxideAgent::core::tools::ToolRegistry;
use OxideAgent::types::AppEvent;
use tokio::sync::mpsc;

#[test]
fn test_orchestrator_new() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    // Use a guard to ensure we always restore the original directory
    struct DirectoryGuard {
        original_dir: std::path::PathBuf,
    }
    
    impl Drop for DirectoryGuard {
        fn drop(&mut self) {
            // Ignore errors when restoring directory, as it might have been deleted
            let _ = std::env::set_current_dir(&self.original_dir);
        }
    }
    
    let original_cwd = std::env::current_dir().unwrap();
    let _guard = DirectoryGuard { original_dir: original_cwd.clone() };

    // Change to temp directory for testing to isolate file operations
    std::env::set_current_dir(&temp_dir).unwrap();

    let config = create_test_config();
    let agent = Agent::new(&config.agent.name, &config.agent.model);
    let tool_registry = ToolRegistry::new();
    let (tx, rx) = mpsc::channel::<AppEvent>(32);

    let orchestrator = Orchestrator::new(
        agent,
        tool_registry,
        config.session.clone(),
        config.no_stream,
        tx,
        rx,
    );

    // After initialization and loading state, the session history should be empty
    // since we're in a fresh temp directory with no session file
    assert_eq!(orchestrator.get_session_history().len(), 0); // Session state history is empty when no file exists

    // Directory will be automatically restored by the guard
}

#[test]
fn test_orchestrator_get_session_history() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    // Use a guard to ensure we always restore the original directory
    struct DirectoryGuard {
        original_dir: std::path::PathBuf,
    }
    
    impl Drop for DirectoryGuard {
        fn drop(&mut self) {
            // Ignore errors when restoring directory, as it might have been deleted
            let _ = std::env::set_current_dir(&self.original_dir);
        }
    }
    
    let original_cwd = std::env::current_dir().unwrap();
    let _guard = DirectoryGuard { original_dir: original_cwd.clone() };

    // Change to temp directory for testing to isolate file operations
    std::env::set_current_dir(&temp_dir).unwrap();

    let config = create_test_config();
    let agent = Agent::new(&config.agent.name, &config.agent.model);
    let tool_registry = ToolRegistry::new();
    let (tx, rx) = mpsc::channel::<AppEvent>(32);

    let orchestrator = Orchestrator::new(
        agent,
        tool_registry,
        config.session.clone(),
        config.no_stream,
        tx,
        rx,
    );

    let history = orchestrator.get_session_history();
    assert_eq!(history.len(), 0); // Session state history is empty when no file exists

    // Directory will be automatically restored by the guard
}

#[test]
fn test_orchestrator_load_state_empty() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    
    // Use a guard to ensure we always restore the original directory
    struct DirectoryGuard {
        original_dir: std::path::PathBuf,
    }
    
    impl Drop for DirectoryGuard {
        fn drop(&mut self) {
            // Ignore errors when restoring directory, as it might have been deleted
            let _ = std::env::set_current_dir(&self.original_dir);
        }
    }
    
    let original_cwd = std::env::current_dir().unwrap();
    let _guard = DirectoryGuard { original_dir: original_cwd.clone() };

    // Change to temp directory for testing to isolate file operations
    std::env::set_current_dir(&temp_dir).unwrap();

    let config = create_test_config();
    let agent = Agent::new(&config.agent.name, &config.agent.model);
    let tool_registry = ToolRegistry::new();
    let (tx, rx) = mpsc::channel::<AppEvent>(32);

    let mut orchestrator = Orchestrator::new(
        agent,
        tool_registry,
        config.session.clone(),
        config.no_stream,
        tx,
        rx,
    );

    let result = orchestrator.load_state();
    assert!(result.is_ok());

    // Directory will be automatically restored by the guard
}

#[tokio::test]
async fn test_orchestrator_list_sessions() {
    // Test the list_sessions function directly
    let result = Orchestrator::list_sessions();
    assert!(result.is_ok());
}

fn create_test_config() -> Config {
    Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: false,
        session: Some("test_session".to_string()),
        list_sessions: false,
        mcp_server: None,
        mcp_auth_token: None,
        interface: InterfaceType::Tui,
    }
}
