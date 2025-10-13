use OxideAgent::core::tools::{
    ReadFileTool, RunShellCommandTool, Tool, ToolProfile, ToolRegistry, WriteFileTool,
};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_write_file_tool_execute() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();

    let tool = WriteFileTool;
    let args = json!({
        "path": file_path_str,
        "content": "Hello, World!"
    });

    // Execute the tool
    let result = tool.execute(&args).await;
    assert!(result.is_ok());

    // Verify the file was created with correct content
    let content = fs::read_to_string(file_path).unwrap();
    assert_eq!(content, "Hello, World!");
}

#[tokio::test]
async fn test_write_file_tool_execute_missing_path() {
    let tool = WriteFileTool;
    let args = json!({
        "content": "Hello, World!"
    });

    // Execute the tool without path argument
    let result = tool.execute(&args).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'path' argument is required")
    );
}

#[tokio::test]
async fn test_write_file_tool_clone_box() {
    let tool = WriteFileTool;
    let cloned_tool = tool.clone_box();

    // Check that the cloned tool has the same properties
    assert_eq!(tool.name(), cloned_tool.name());
    assert_eq!(tool.description(), cloned_tool.description());
    assert_eq!(tool.parameters(), cloned_tool.parameters());
    assert_eq!(tool.profile(), cloned_tool.profile());

    // Test that the cloned tool works functionally
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("clone_test.txt");
    let file_path_str = file_path.to_str().unwrap();

    let args = json!({
        "path": file_path_str,
        "content": "Clone test"
    });

    let result = cloned_tool.execute(&args).await;
    assert!(result.is_ok());

    let content = fs::read_to_string(file_path).unwrap();
    assert_eq!(content, "Clone test");
}

#[tokio::test]
async fn test_read_file_tool_execute() {
    // Create a temporary directory and file for testing
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("read_test.txt");
    let file_path_str = file_path.to_str().unwrap();

    // Write some content to the test file
    fs::write(&file_path, "Test content").unwrap();

    let tool = ReadFileTool;
    let args = json!({
        "path": file_path_str
    });

    // Execute the tool
    let result = tool.execute(&args).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Test content");
}

#[tokio::test]
async fn test_read_file_tool_execute_missing_path() {
    let tool = ReadFileTool;
    let args = json!({});

    // Execute the tool without path argument
    let result = tool.execute(&args).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'path' argument is required")
    );
}

#[tokio::test]
async fn test_read_file_tool_execute_nonexistent_file() {
    let tool = ReadFileTool;
    let args = json!({
        "path": "/nonexistent/file.txt"
    });

    // Execute the tool with a non-existent file
    let result = tool.execute(&args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_file_tool_clone_box() {
    let tool = ReadFileTool;
    let cloned_tool = tool.clone_box();

    // Check that the cloned tool has the same properties
    assert_eq!(tool.name(), cloned_tool.name());
    assert_eq!(tool.description(), cloned_tool.description());
    assert_eq!(tool.parameters(), cloned_tool.parameters());
    assert_eq!(tool.profile(), cloned_tool.profile());

    // Test that the cloned tool works functionally
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("cloned_read_test.txt");
    let file_path_str = file_path.to_str().unwrap();

    // Write some content to the test file
    fs::write(&file_path, "Cloned tool test").unwrap();

    let args = json!({
        "path": file_path_str
    });

    let result = cloned_tool.execute(&args).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Cloned tool test");
}

#[tokio::test]
async fn test_run_shell_command_tool_execute() {
    let tool = RunShellCommandTool;
    let args = json!({
        "command": "echo 'Hello, World!'"
    });

    // Execute the tool
    let result = tool.execute(&args).await;
    assert!(result.is_ok());
    // Note: echo may add a newline, so we check for content that includes our expected output
    let output = result.unwrap();
    assert!(output.contains("Hello, World!"));
}

#[tokio::test]
async fn test_run_shell_command_tool_execute_missing_command() {
    let tool = RunShellCommandTool;
    let args = json!({});

    // Execute the tool without command argument
    let result = tool.execute(&args).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'command' argument is required")
    );
}

#[tokio::test]
async fn test_run_shell_command_tool_execute_failed_command() {
    let tool = RunShellCommandTool;
    let args = json!({
        "command": "nonexistent_command_xyz"
    });

    // Execute the tool with a command that should fail
    let result = tool.execute(&args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_run_shell_command_tool_clone_box() {
    let tool = RunShellCommandTool;
    let cloned_tool = tool.clone_box();

    // Check that the cloned tool has the same properties
    assert_eq!(tool.name(), cloned_tool.name());
    assert_eq!(tool.description(), cloned_tool.description());
    assert_eq!(tool.parameters(), cloned_tool.parameters());
    assert_eq!(tool.profile(), cloned_tool.profile());

    // Test that the cloned tool works functionally
    let args = json!({
        "command": "echo 'Cloned command test'"
    });

    let result = cloned_tool.execute(&args).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Cloned command test"));
}

#[test]
fn test_tool_registry() {
    let mut registry = ToolRegistry::new();

    // Add tools to the registry
    registry.add_tool(Box::new(WriteFileTool));
    registry.add_tool(Box::new(ReadFileTool));

    // Test getting a tool by name
    let write_tool = registry.get_tool("write_file");
    assert!(write_tool.is_some());
    assert_eq!(write_tool.unwrap().name(), "write_file");

    let read_tool = registry.get_tool("read_file");
    assert!(read_tool.is_some());
    assert_eq!(read_tool.unwrap().name(), "read_file");

    // Test getting a non-existent tool
    let nonexistent_tool = registry.get_tool("nonexistent");
    assert!(nonexistent_tool.is_none());

    // Test getting all tool definitions
    let definitions = registry.definitions();
    assert_eq!(definitions.len(), 2);

    // Test getting tool definitions with profiles
    let file_profile_definitions = registry.definitions_with_profiles(&[ToolProfile::File]);
    assert_eq!(file_profile_definitions.len(), 2);
}

#[test]
fn test_tool_registry_clone() {
    let mut registry = ToolRegistry::new();

    // Add tools to the registry
    registry.add_tool(Box::new(WriteFileTool));
    registry.add_tool(Box::new(ReadFileTool));

    // Clone the registry
    let cloned_registry = registry.clone_registry();

    // Verify the cloned registry has the same tools
    let write_tool = cloned_registry.get_tool("write_file");
    assert!(write_tool.is_some());
    assert_eq!(write_tool.unwrap().name(), "write_file");

    let read_tool = cloned_registry.get_tool("read_file");
    assert!(read_tool.is_some());
    assert_eq!(read_tool.unwrap().name(), "read_file");
}
