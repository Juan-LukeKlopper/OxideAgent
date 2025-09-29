//! Mock objects for testing external dependencies.

use crate::types::{AppEvent, ChatMessage, Tool as ApiTool};
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::mpsc;

// Mock for the Ollama API client
#[allow(dead_code)]  // Fields and methods are used in tests
pub struct MockOllamaClient {
    pub responses: Vec<serde_json::Value>,
    pub call_count: usize,
}

impl Default for MockOllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]  // Methods are used in tests and form part of the public API
impl MockOllamaClient {
    pub fn new() -> Self {
        Self {
            responses: Vec::new(),
            call_count: 0,
        }
    }

    pub fn add_response(&mut self, response: serde_json::Value) {
        self.responses.push(response);
    }

    pub async fn send_chat(
        &mut self,
        _model: &str,
        _history: &[ChatMessage],
        _tools: &[ApiTool],
        _stream: bool,
        tx: mpsc::Sender<AppEvent>,
    ) -> anyhow::Result<Option<ChatMessage>> {
        self.call_count += 1;

        if self.call_count > self.responses.len() {
            // Default response if none provided
            let default_response = serde_json::json!({
                "message": {
                    "content": "Default mock response",
                    "role": "assistant"
                }
            });
            self.responses.push(default_response);
        }

        let response = &self.responses[self.call_count - 1];
        let content = response["message"]["content"]
            .as_str()
            .unwrap_or("Default mock response")
            .to_string();

        // Send streaming events if in streaming mode
        for c in content.chars() {
            tx.send(AppEvent::AgentStreamChunk(c.to_string())).await?;
        }
        tx.send(AppEvent::AgentStreamEnd).await?;

        Ok(Some(ChatMessage::assistant(&content)))
    }
}

// Mock for file system operations
pub struct MockFileSystem {
    pub files: HashMap<String, String>,
    pub read_results: HashMap<String, Result<String, String>>,
    pub write_results: HashMap<String, Result<(), String>>,
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]  // Methods are used in tests and form part of the public API
impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            read_results: HashMap::new(),
            write_results: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.to_string());
    }

    pub fn read_file(&mut self, path: &str) -> Result<String, String> {
        // Check if there's a specific result for this path
        if let Some(result) = self.read_results.get(path) {
            return result.clone();
        }

        // Otherwise, look in the mock file system
        match self.files.get(path) {
            Some(content) => Ok(content.clone()),
            None => Err(format!("File '{}' not found", path)),
        }
    }

    pub fn write_file(&mut self, path: &str, content: &str) -> Result<(), String> {
        // Check if there's a specific result for this path
        if let Some(result) = self.write_results.get(path) {
            return result.clone();
        }

        // Otherwise, write to the mock file system
        self.files.insert(path.to_string(), content.to_string());
        Ok(())
    }

    pub fn set_read_result(&mut self, path: &str, result: Result<String, String>) {
        self.read_results.insert(path.to_string(), result);
    }

    pub fn set_write_result(&mut self, path: &str, result: Result<(), String>) {
        self.write_results.insert(path.to_string(), result);
    }
}

// Mock for shell command execution
pub struct MockShellExecutor {
    pub command_outputs: HashMap<String, Result<String, String>>,
    pub call_log: Vec<String>,
}

impl Default for MockShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]  // Methods are used in tests and form part of the public API
impl MockShellExecutor {
    pub fn new() -> Self {
        Self {
            command_outputs: HashMap::new(),
            call_log: Vec::new(),
        }
    }

    pub fn add_command_output(&mut self, command: &str, output: Result<String, String>) {
        self.command_outputs.insert(command.to_string(), output);
    }

    pub fn execute_command(&mut self, command: &str) -> Result<String, String> {
        self.call_log.push(command.to_string());

        // Check if there's a specific result for this command
        if let Some(output) = self.command_outputs.get(command) {
            output.clone()
        } else {
            // Default successful response
            Ok(format!("Command '{}' executed successfully", command))
        }
    }

    pub fn get_call_log(&self) -> Vec<String> {
        self.call_log.clone()
    }

    pub fn reset_call_log(&mut self) {
        self.call_log.clear();
    }
}

// Mock implementations of the tools that use the mock file system and shell executor
use crate::core::tools::{Tool as CoreTool, ToolProfile};

// Mock for write file tool
pub struct MockWriteFileTool {
    pub mock_file_system: std::sync::Arc<std::sync::Mutex<MockFileSystem>>,
}

#[allow(dead_code)]  // Methods are used in tests and form part of the public API
impl MockWriteFileTool {
    pub fn new(mock_file_system: std::sync::Arc<std::sync::Mutex<MockFileSystem>>) -> Self {
        Self { mock_file_system }
    }
}

impl CoreTool for MockWriteFileTool {
    fn name(&self) -> String {
        "write_file".to_string()
    }

    fn description(&self) -> String {
        "Write content to a file".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn profile(&self) -> ToolProfile {
        ToolProfile::File
    }

    fn execute(&self, args: &Value) -> anyhow::Result<String> {
        let path = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        if path.is_empty() {
            return Err(anyhow::anyhow!("'path' argument is required"));
        }

        let mut fs = self.mock_file_system.lock().unwrap();
        match fs.write_file(path, content) {
            Ok(()) => Ok(format!("File '{}' written successfully.", path)),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}

// Mock for read file tool
pub struct MockReadFileTool {
    pub mock_file_system: std::sync::Arc<std::sync::Mutex<MockFileSystem>>,
}

#[allow(dead_code)]  // Methods are used in tests and form part of the public API
impl MockReadFileTool {
    pub fn new(mock_file_system: std::sync::Arc<std::sync::Mutex<MockFileSystem>>) -> Self {
        Self { mock_file_system }
    }
}

impl CoreTool for MockReadFileTool {
    fn name(&self) -> String {
        "read_file".to_string()
    }

    fn description(&self) -> String {
        "Read content from a file".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    fn profile(&self) -> ToolProfile {
        ToolProfile::File
    }

    fn execute(&self, args: &Value) -> anyhow::Result<String> {
        let path = args["path"].as_str().unwrap_or("");
        if path.is_empty() {
            return Err(anyhow::anyhow!("'path' argument is required"));
        }

        let mut fs = self.mock_file_system.lock().unwrap();
        match fs.read_file(path) {
            Ok(content) => Ok(content),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}

// Mock for run shell command tool
pub struct MockRunShellCommandTool {
    pub mock_shell_executor: std::sync::Arc<std::sync::Mutex<MockShellExecutor>>,
}

#[allow(dead_code)]  // Methods are used in tests and form part of the public API
impl MockRunShellCommandTool {
    pub fn new(mock_shell_executor: std::sync::Arc<std::sync::Mutex<MockShellExecutor>>) -> Self {
        Self {
            mock_shell_executor,
        }
    }
}

impl CoreTool for MockRunShellCommandTool {
    fn name(&self) -> String {
        "run_shell_command".to_string()
    }

    fn description(&self) -> String {
        "Run a shell command".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to run"
                }
            },
            "required": ["command"]
        })
    }

    fn profile(&self) -> ToolProfile {
        ToolProfile::Shell
    }

    fn execute(&self, args: &Value) -> anyhow::Result<String> {
        let command = args["command"].as_str().unwrap_or("");
        if command.is_empty() {
            return Err(anyhow::anyhow!("'command' argument is required"));
        }

        let mut shell = self.mock_shell_executor.lock().unwrap();
        match shell.execute_command(command) {
            Ok(output) => Ok(output),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_file_system() {
        let mut fs = MockFileSystem::new();
        fs.add_file("test.txt", "Hello, world!");

        // Test successful read
        let result = fs.read_file("test.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");

        // Test reading non-existent file
        let result = fs.read_file("nonexistent.txt");
        assert!(result.is_err());

        // Test writing a file
        let write_result = fs.write_file("new_file.txt", "New content");
        assert!(write_result.is_ok());

        // Verify the file was written
        let read_result = fs.read_file("new_file.txt");
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), "New content");
    }

    #[test]
    fn test_mock_shell_executor() {
        let mut shell = MockShellExecutor::new();
        shell.add_command_output("echo hello", Ok("hello".to_string()));
        shell.add_command_output("invalid_command", Err("Command not found".to_string()));

        // Test successful command
        let result = shell.execute_command("echo hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");

        // Test failing command
        let result = shell.execute_command("invalid_command");
        assert!(result.is_err());

        // Test logging
        assert_eq!(
            shell.get_call_log(),
            vec!["echo hello".to_string(), "invalid_command".to_string()]
        );
    }

    #[tokio::test]
    async fn test_mock_ollama_client() {
        let mut client = MockOllamaClient::new();

        // Add a response
        client.add_response(serde_json::json!({
            "message": {
                "content": "Hello from mock Ollama",
                "role": "assistant"
            }
        }));

        // Create a channel for events
        let (tx, mut rx) = mpsc::channel::<AppEvent>(32);

        // Call the mock client
        let result = client.send_chat("test-model", &[], &[], true, tx).await;

        assert!(result.is_ok());

        // Verify the streaming events were sent
        let mut received_chunks = Vec::new();
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::AgentStreamChunk(chunk) => received_chunks.push(chunk),
                _ => {}
            }
        }

        assert_eq!(received_chunks.join(""), "Hello from mock Ollama");
        assert_eq!(client.call_count, 1);
    }

    #[test]
    fn test_mock_write_file_tool() {
        let mock_fs = std::sync::Arc::new(std::sync::Mutex::new(MockFileSystem::new()));
        let tool = MockWriteFileTool::new(mock_fs.clone());

        // Test tool properties
        assert_eq!(tool.name(), "write_file");
        assert_eq!(tool.description(), "Write content to a file");
        assert_eq!(tool.profile(), ToolProfile::File);

        // Test execution
        let args = json!({
            "path": "test.txt",
            "content": "Hello, mock!"
        });

        let result = tool.execute(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "File 'test.txt' written successfully.");

        // Verify the file was written to the mock file system
        let mut fs = mock_fs.lock().unwrap();
        assert_eq!(fs.read_file("test.txt").unwrap(), "Hello, mock!");
    }

    #[test]
    fn test_mock_read_file_tool() {
        let mock_fs = std::sync::Arc::new(std::sync::Mutex::new(MockFileSystem::new()));

        // Pre-populate the mock file system
        {
            let mut fs = mock_fs.lock().unwrap();
            fs.add_file("test.txt", "Hello, mock!");
        }

        let tool = MockReadFileTool::new(mock_fs);

        // Test tool properties
        assert_eq!(tool.name(), "read_file");
        assert_eq!(tool.description(), "Read content from a file");
        assert_eq!(tool.profile(), ToolProfile::File);

        // Test execution
        let args = json!({
            "path": "test.txt"
        });

        let result = tool.execute(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, mock!");
    }

    #[test]
    fn test_mock_run_shell_command_tool() {
        let mock_shell = std::sync::Arc::new(std::sync::Mutex::new(MockShellExecutor::new()));
        let tool = MockRunShellCommandTool::new(mock_shell);

        // Test tool properties
        assert_eq!(tool.name(), "run_shell_command");
        assert_eq!(tool.description(), "Run a shell command");
        assert_eq!(tool.profile(), ToolProfile::Shell);

        // Test execution
        let args = json!({
            "command": "echo hello"
        });

        let result = tool.execute(&args);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("echo hello"));
    }
}
