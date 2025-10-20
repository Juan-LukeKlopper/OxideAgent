//! Mock objects for testing external dependencies.

use crate::core::tools::Tool;
use crate::types::{AppEvent, ChatMessage, Tool as ApiTool};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::mpsc;

// Mock for the Ollama API client
#[allow(dead_code)] // Fields and methods are used in tests
pub struct MockOllamaClient {
    pub responses: Vec<serde_json::Value>,
    pub call_count: usize,
}

impl Default for MockOllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // Methods are used in tests and form part of the public API
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

#[allow(dead_code)] // Methods are used in tests and form part of the public API
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

#[allow(dead_code)] // Methods are used in tests and form part of the public API
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

#[allow(dead_code)] // Methods are used in tests and form part of the public API
impl MockWriteFileTool {
    pub fn new(mock_file_system: std::sync::Arc<std::sync::Mutex<MockFileSystem>>) -> Self {
        Self { mock_file_system }
    }
}

#[async_trait]
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

    async fn execute(&self, args: &Value) -> anyhow::Result<String> {
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

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(MockWriteFileTool {
            mock_file_system: self.mock_file_system.clone(),
        })
    }
}

// Mock for read file tool
pub struct MockReadFileTool {
    pub mock_file_system: std::sync::Arc<std::sync::Mutex<MockFileSystem>>,
}

#[allow(dead_code)] // Methods are used in tests and form part of the public API
impl MockReadFileTool {
    pub fn new(mock_file_system: std::sync::Arc<std::sync::Mutex<MockFileSystem>>) -> Self {
        Self { mock_file_system }
    }
}

#[async_trait]
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

    async fn execute(&self, args: &Value) -> anyhow::Result<String> {
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

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(MockReadFileTool {
            mock_file_system: self.mock_file_system.clone(),
        })
    }
}

// Mock for run shell command tool
pub struct MockRunShellCommandTool {
    pub mock_shell_executor: std::sync::Arc<std::sync::Mutex<MockShellExecutor>>,
}

#[allow(dead_code)] // Methods are used in tests and form part of the public API
impl MockRunShellCommandTool {
    pub fn new(mock_shell_executor: std::sync::Arc<std::sync::Mutex<MockShellExecutor>>) -> Self {
        Self {
            mock_shell_executor,
        }
    }
}

#[async_trait]
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

    async fn execute(&self, args: &Value) -> anyhow::Result<String> {
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

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(MockRunShellCommandTool {
            mock_shell_executor: self.mock_shell_executor.clone(),
        })
    }
}
