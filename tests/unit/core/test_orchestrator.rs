//! Unit tests for the orchestrator module using mock objects.

use OxideAgent::config::{AgentConfig, AgentType, InterfaceType, OxideConfig as Config};
use OxideAgent::core::orchestrator::Orchestrator;
use OxideAgent::core::tools::ToolRegistry;
use OxideAgent::types::AppEvent;
use tokio::sync::mpsc;

use crate::utils::CWD_MUTEX;

#[tokio::test]
async fn test_orchestrator_new() {
    let _lock = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
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
    let _guard = DirectoryGuard {
        original_dir: original_cwd.clone(),
    };

    // Change to temp directory for testing to isolate file operations
    std::env::set_current_dir(&temp_dir).unwrap();

    let config = create_test_config();
    let system_prompt = &config.agent.system_prompt;
    let model = config.agent.model.clone();
    let tool_registry = ToolRegistry::new();
    let (tx, rx) = mpsc::channel::<AppEvent>(32);

    let orchestrator = Orchestrator::new(
        system_prompt,
        tool_registry,
        config.session.clone(),
        config.no_stream,
        tx,
        rx,
        model,
        config.llm.clone(),
    );

    // After initialization and loading state, the session history should be empty
    // since we're in a fresh temp directory with no session file
    assert_eq!(orchestrator.get_session_history().len(), 0); // Session state history is empty when no file exists

    // Directory will be automatically restored by the guard
}

#[tokio::test]
async fn test_orchestrator_get_session_history() {
    let _lock = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
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
    let _guard = DirectoryGuard {
        original_dir: original_cwd.clone(),
    };

    // Change to temp directory for testing to isolate file operations
    std::env::set_current_dir(&temp_dir).unwrap();

    let config = create_test_config();
    let system_prompt = &config.agent.system_prompt;
    let model = config.agent.model.clone();
    let tool_registry = ToolRegistry::new();
    let (tx, rx) = mpsc::channel::<AppEvent>(32);

    let orchestrator = Orchestrator::new(
        system_prompt,
        tool_registry,
        config.session.clone(),
        config.no_stream,
        tx,
        rx,
        model,
        config.llm.clone(),
    );

    let history = orchestrator.get_session_history();
    assert_eq!(history.len(), 0); // Session state history is empty when no file exists

    // Directory will be automatically restored by the guard
}

#[tokio::test]
async fn test_orchestrator_load_state_empty() {
    let _lock = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
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
    let _guard = DirectoryGuard {
        original_dir: original_cwd.clone(),
    };

    // Change to temp directory for testing to isolate file operations
    std::env::set_current_dir(&temp_dir).unwrap();

    let config = create_test_config();
    let system_prompt = &config.agent.system_prompt;
    let model = config.agent.model.clone();
    let tool_registry = ToolRegistry::new();
    let (tx, rx) = mpsc::channel::<AppEvent>(32);

    let mut orchestrator = Orchestrator::new(
        system_prompt,
        tool_registry,
        config.session.clone(),
        config.no_stream,
        tx,
        rx,
        model,
        config.llm.clone(),
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
            api_base: String::new(),
            api_key: None,
            model: Some("qwen3:4b".to_string()),
        },
        multi_agent: OxideAgent::config::MultiAgentConfig::default(),
    }
}
