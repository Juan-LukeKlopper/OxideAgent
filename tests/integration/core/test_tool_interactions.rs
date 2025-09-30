//! Integration tests for tool execution workflows.

use OxideAgent::config::{AgentConfig, AgentType, InterfaceType, OxideConfig as Config};
use OxideAgent::core::container::Container;
use OxideAgent::core::tools::{
    ReadFileTool, RunShellCommandTool, Tool, ToolRegistry, WriteFileTool,
};
use OxideAgent::types::{AppEvent, ToolCall, ToolFunction};
use serde_json::json;
use std::fs;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_tool_execution_workflow() {
    // Test full tool execution workflow
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: true,
        session: Some("tool_test_session".to_string()),
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

    // Test WriteFileTool directly
    let write_tool = WriteFileTool;
    let write_args = json!({
        "path": "test_tool_workflow.txt",
        "content": "Hello from tool workflow test!"
    });
    let write_result = write_tool.execute(&write_args);
    assert!(write_result.is_ok());

    // Verify the file was created with correct content
    assert!(std::path::Path::new("test_tool_workflow.txt").exists());
    let content = fs::read_to_string("test_tool_workflow.txt").unwrap();
    assert_eq!(content, "Hello from tool workflow test!");

    // Test ReadFileTool directly
    let read_tool = ReadFileTool;
    let read_args = json!({
        "path": "test_tool_workflow.txt"
    });
    let read_result = read_tool.execute(&read_args);
    assert!(read_result.is_ok());
    assert_eq!(read_result.unwrap(), "Hello from tool workflow test!");

    // Test RunShellCommandTool directly
    let shell_tool = RunShellCommandTool;
    let shell_args = json!({
        "command": "echo 'Hello from shell tool!'"
    });
    let shell_result = shell_tool.execute(&shell_args);
    assert!(shell_result.is_ok());
    let shell_output = shell_result.unwrap();
    assert!(shell_output.trim() == "Hello from shell tool!");

    // Clean up
    let _ = fs::remove_file("test_tool_workflow.txt");
}

#[tokio::test]
async fn test_tool_registry_integration() {
    // Test that the tool registry works correctly with orchestrator
    let mut tool_registry = ToolRegistry::new();

    // Register tools
    tool_registry.add_tool(Box::new(WriteFileTool));
    tool_registry.add_tool(Box::new(ReadFileTool));
    tool_registry.add_tool(Box::new(RunShellCommandTool));

    // Verify tools are registered
    assert_eq!(tool_registry.definitions().len(), 3);

    // Test that we can get each tool by name
    let write_tool = tool_registry.get_tool("write_file");
    assert!(write_tool.is_some());
    assert_eq!(write_tool.unwrap().name(), "write_file");

    let read_tool = tool_registry.get_tool("read_file");
    assert!(read_tool.is_some());
    assert_eq!(read_tool.unwrap().name(), "read_file");

    let shell_tool = tool_registry.get_tool("run_shell_command");
    assert!(shell_tool.is_some());
    assert_eq!(shell_tool.unwrap().name(), "run_shell_command");

    // Test that a non-existent tool returns None
    let nonexistent_tool = tool_registry.get_tool("nonexistent_tool");
    assert!(nonexistent_tool.is_none());
}

#[tokio::test]
async fn test_write_file_tool_functionality() {
    let tool = WriteFileTool;

    // Test successful file write
    let args = json!({
        "path": "test_write_functionality.txt",
        "content": "Test content for functionality test"
    });

    let result = tool.execute(&args);
    assert!(result.is_ok());

    // Verify file was created with correct content
    assert!(std::path::Path::new("test_write_functionality.txt").exists());
    let content = fs::read_to_string("test_write_functionality.txt").unwrap();
    assert_eq!(content, "Test content for functionality test");

    // Test with special characters
    let args_unicode = json!({
        "path": "test_unicode.txt",
        "content": "Hello, 世界! 🌍"
    });

    let result = tool.execute(&args_unicode);
    assert!(result.is_ok());

    // Verify unicode content
    let unicode_content = fs::read_to_string("test_unicode.txt").unwrap();
    assert_eq!(unicode_content, "Hello, 世界! 🌍");

    // Clean up
    let _ = fs::remove_file("test_write_functionality.txt");
    let _ = fs::remove_file("test_unicode.txt");
}

#[tokio::test]
async fn test_read_file_tool_functionality() {
    // Create test files
    fs::write("test_read_normal.txt", "Normal file content").unwrap();
    fs::write("test_read_unicode.txt", "Unicode content: 世界 🌍").unwrap();
    fs::write("test_read_empty.txt", "").unwrap();

    let tool = ReadFileTool;

    // Test reading normal file
    let args = json!({
        "path": "test_read_normal.txt"
    });
    let result = tool.execute(&args);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Normal file content");

    // Test reading unicode file
    let args = json!({
        "path": "test_read_unicode.txt"
    });
    let result = tool.execute(&args);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Unicode content: 世界 🌍");

    // Test reading empty file
    let args = json!({
        "path": "test_read_empty.txt"
    });
    let result = tool.execute(&args);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");

    // Test reading non-existent file (should fail)
    let args = json!({
        "path": "nonexistent_file.txt"
    });
    let result = tool.execute(&args);
    assert!(result.is_err());

    // Clean up
    let _ = fs::remove_file("test_read_normal.txt");
    let _ = fs::remove_file("test_read_unicode.txt");
    let _ = fs::remove_file("test_read_empty.txt");
}

#[tokio::test]
async fn test_run_shell_command_tool_functionality() {
    let tool = RunShellCommandTool;

    // Test echo command
    let args = json!({
        "command": "echo 'Hello, World!'"
    });
    let result = tool.execute(&args);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.trim(), "Hello, World!");

    // Test ls command (should not fail)
    let args = json!({
        "command": "ls"
    });
    let result = tool.execute(&args);
    assert!(result.is_ok());

    // Test command that fails
    let args = json!({
        "command": "nonexistent_command_that_does_not_exist"
    });
    let result = tool.execute(&args);
    assert!(result.is_err());

    // Test command with arguments
    let args = json!({
        "command": "printf 'Test %s' 'command'"
    });
    let result = tool.execute(&args);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.trim(), "Test command");
}
