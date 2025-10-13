//! Unit tests for the tools module using mock objects.

use OxideAgent::core::mocks::{
    MockFileSystem, MockReadFileTool, MockRunShellCommandTool, MockShellExecutor, MockWriteFileTool,
};
use OxideAgent::core::tools::{Tool, ToolProfile, ToolRegistry};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_tool_registry_new() {
    let registry = ToolRegistry::new();
    assert_eq!(registry.definitions().len(), 0);
}

#[test]
fn test_tool_registry_add_tool() {
    let mut registry = ToolRegistry::new();
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let tool = MockWriteFileTool::new(mock_fs);

    registry.add_tool(Box::new(tool));
    assert_eq!(registry.definitions().len(), 1);

    let definitions = registry.definitions();
    assert_eq!(definitions[0].function.name, "write_file");
}

#[test]
fn test_tool_registry_get_tool() {
    let mut registry = ToolRegistry::new();
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let tool = MockWriteFileTool::new(mock_fs);

    registry.add_tool(Box::new(tool));

    let retrieved_tool = registry.get_tool("write_file");
    assert!(retrieved_tool.is_some());
    assert_eq!(retrieved_tool.unwrap().name(), "write_file");

    let non_existent_tool = registry.get_tool("non_existent");
    assert!(non_existent_tool.is_none());
}

#[test]
fn test_tool_registry_definitions() {
    let mut registry = ToolRegistry::new();
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let mock_shell = Arc::new(Mutex::new(MockShellExecutor::new()));

    registry.add_tool(Box::new(MockWriteFileTool::new(mock_fs.clone())));
    registry.add_tool(Box::new(MockReadFileTool::new(mock_fs)));
    registry.add_tool(Box::new(MockRunShellCommandTool::new(mock_shell)));

    let definitions = registry.definitions();
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
fn test_tool_registry_definitions_with_profiles() {
    let mut registry = ToolRegistry::new();
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let mock_shell = Arc::new(Mutex::new(MockShellExecutor::new()));

    registry.add_tool(Box::new(MockWriteFileTool::new(mock_fs.clone())));
    registry.add_tool(Box::new(MockReadFileTool::new(mock_fs)));
    registry.add_tool(Box::new(MockRunShellCommandTool::new(mock_shell)));

    // Test filtering for File tools
    let file_tools = registry.definitions_with_profiles(&[ToolProfile::File]);
    assert_eq!(file_tools.len(), 2); // MockWriteFileTool and MockReadFileTool

    let file_tool_names: Vec<String> = file_tools.iter().map(|t| t.function.name.clone()).collect();
    assert!(file_tool_names.contains(&"write_file".to_string()));
    assert!(file_tool_names.contains(&"read_file".to_string()));

    // Test filtering for Shell tools
    let shell_tools = registry.definitions_with_profiles(&[ToolProfile::Shell]);
    assert_eq!(shell_tools.len(), 1); // MockRunShellCommandTool

    let shell_tool_names: Vec<String> = shell_tools
        .iter()
        .map(|t| t.function.name.clone())
        .collect();
    assert!(shell_tool_names.contains(&"run_shell_command".to_string()));
}

#[test]
fn test_tool_definition() {
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let tool = MockWriteFileTool::new(mock_fs);
    let definition = tool.definition();

    assert_eq!(definition.function.name, tool.name());
    assert_eq!(definition.function.description, tool.description());
    assert_eq!(definition.function.parameters, tool.parameters());
}

#[tokio::test]
async fn test_mock_write_file_tool() {
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let tool = MockWriteFileTool::new(mock_fs.clone());

    assert_eq!(tool.name(), "write_file");
    assert_eq!(tool.description(), "Write content to a file");
    assert_eq!(tool.profile(), ToolProfile::File);

    // Test parameters schema
    let params = tool.parameters();
    assert!(params.is_object());
    assert_eq!(params["type"], "object");

    // Test execution
    let args = json!({
        "path": "test_write.txt",
        "content": "Hello, World!"
    });

    let result = tool.execute(&args).await;
    assert!(result.is_ok());

    // Verify file was created with correct content in the mock file system
    let mut fs = mock_fs.lock().unwrap();
    assert_eq!(fs.read_file("test_write.txt").unwrap(), "Hello, World!");
}

#[tokio::test]
async fn test_mock_write_file_tool_missing_args() {
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let tool = MockWriteFileTool::new(mock_fs);

    // Test with missing path
    let args = json!({
        "content": "Hello, World!"
    });

    let result = tool.execute(&args).await;
    assert!(result.is_err());

    // Test with missing content (but existing path)
    let args = json!({
        "path": "test_write.txt"
    });

    let result = tool.execute(&args).await;
    assert!(result.is_ok()); // Content is optional in this implementation

    // Test with empty path
    let args = json!({
        "path": "",
        "content": "Hello, World!"
    });

    let result = tool.execute(&args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mock_read_file_tool() {
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));

    // Pre-populate the mock file system
    {
        let mut fs = mock_fs.lock().unwrap();
        fs.add_file("test_read.txt", "Hello, World!");
    }

    let tool = MockReadFileTool::new(mock_fs);

    assert_eq!(tool.name(), "read_file");
    assert_eq!(tool.description(), "Read content from a file");
    assert_eq!(tool.profile(), ToolProfile::File);

    // Test parameters schema
    let params = tool.parameters();
    assert!(params.is_object());
    assert_eq!(params["type"], "object");

    // Test execution
    let args = json!({
        "path": "test_read.txt"
    });

    let result = tool.execute(&args).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, World!");
}

#[tokio::test]
async fn test_mock_read_file_tool_missing_args() {
    let mock_fs = Arc::new(Mutex::new(MockFileSystem::new()));
    let tool = MockReadFileTool::new(mock_fs);

    // Test with missing path
    let args = json!({});

    let result = tool.execute(&args).await;
    assert!(result.is_err());

    // Test with empty path
    let args = json!({
        "path": ""
    });

    let result = tool.execute(&args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mock_run_shell_command_tool() {
    let mock_shell = Arc::new(Mutex::new(MockShellExecutor::new()));
    let tool = MockRunShellCommandTool::new(mock_shell);

    assert_eq!(tool.name(), "run_shell_command");
    assert_eq!(tool.description(), "Run a shell command");
    assert_eq!(tool.profile(), ToolProfile::Shell);

    // Test parameters schema
    let params = tool.parameters();
    assert!(params.is_object());
    assert_eq!(params["type"], "object");

    // Test execution
    let args = json!({
        "command": "echo 'Hello, World!'"
    });

    let result = tool.execute(&args).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("echo 'Hello, World!'"));
}

#[tokio::test]
async fn test_mock_run_shell_command_tool_missing_args() {
    let mock_shell = Arc::new(Mutex::new(MockShellExecutor::new()));
    let tool = MockRunShellCommandTool::new(mock_shell);

    // Test with missing command
    let args = json!({});

    let result = tool.execute(&args).await;
    assert!(result.is_err());

    // Test with empty command
    let args = json!({
        "command": ""
    });

    let result = tool.execute(&args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mock_run_shell_command_tool_error() {
    let mock_shell = Arc::new(Mutex::new(MockShellExecutor::new()));

    // Set up an expected error for a specific command
    {
        let mut shell = mock_shell.lock().unwrap();
        shell.add_command_output("nonexistent_command", Err("Command not found".to_string()));
    }

    let tool = MockRunShellCommandTool::new(mock_shell);

    // Test with failing command
    let args = json!({
        "command": "nonexistent_command"
    });

    let result = tool.execute(&args).await;
    assert!(result.is_err());
}
