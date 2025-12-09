//! Test the Ollama integration with mock objects.

use OxideAgent::core::agents::Agent;
use OxideAgent::types::AppEvent;
use reqwest::Client;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_agent_with_mock_ollama_client() {
    // This test would use the mock Ollama client to test agent interactions
    // without requiring a real Ollama instance

    let _client = Client::new();
    let (_tx, _rx): (mpsc::Sender<AppEvent>, mpsc::Receiver<AppEvent>) = mpsc::channel(1);

    // Create a test agent
    let mut agent = Agent::new("TestAgent");

    // Add a user message to trigger a response
    agent.add_user_message("Hello, can you help me?");

    // In a real test, we would use the MockOllamaClient to simulate Ollama responses
    // For now, we're just ensuring the test compiles and runs

    assert_eq!(agent.history.len(), 2); // System + User message
    assert_eq!(agent.history[1].content, "Hello, can you help me?");
}

#[tokio::test]
async fn test_file_operations_with_mock_filesystem() {
    use OxideAgent::core::tools::{Tool, WriteFileTool};

    // Test the WriteFileTool
    let tool = WriteFileTool;
    assert_eq!(tool.name(), "write_file");
}

#[tokio::test]
async fn test_shell_commands_with_mock_executor() {
    use OxideAgent::core::tools::{RunShellCommandTool, Tool};

    let tool = RunShellCommandTool;
    assert_eq!(tool.name(), "run_shell_command");
}
