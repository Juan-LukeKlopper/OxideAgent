use crate::types::Tool as ApiTool;
use serde_json::{Value, json};
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]  // Variants may be used in different configurations
pub enum ToolProfile {
    File,
    Shell,
    Web,
    Generic,
}

// The main trait for any tool that can be executed by the agent.
#[allow(dead_code)]  // Trait methods are part of the public API
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn parameters(&self) -> Value;
    fn profile(&self) -> ToolProfile;
    fn execute(&self, args: &Value) -> anyhow::Result<String>;

    // Provides the full tool definition for the Ollama API.
    fn definition(&self) -> ApiTool {
        ApiTool::new(&self.name(), &self.description(), self.parameters())
    }
}

// A registry to hold all available tools.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

#[allow(dead_code)]  // Methods are used in the application and form part of the public API
impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<ApiTool> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    pub fn definitions_with_profiles(&self, profiles: &[ToolProfile]) -> Vec<ApiTool> {
        self.tools
            .iter()
            .filter(|t| profiles.contains(&t.profile()))
            .map(|t| t.definition())
            .collect()
    }

    // Clone method that creates a new registry with the same tool definitions
    pub fn clone_registry(&self) -> Self {
        // Since tools are stateless, we can recreate the registry with the same tools
        // In a real implementation, we'd need to register the tools again
        // For now, we'll return an empty registry and let the caller register the tools
        ToolRegistry::new()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Tool for writing content to a file.
pub struct WriteFileTool;

impl Tool for WriteFileTool {
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
        fs::write(path, content)?;
        Ok(format!("File '{}' written successfully.", path))
    }
}

// Tool for reading content from a file.
pub struct ReadFileTool;

impl Tool for ReadFileTool {
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
        let content = fs::read_to_string(path)?;
        Ok(content)
    }
}

// Tool for running a shell command.
pub struct RunShellCommandTool;

impl Tool for RunShellCommandTool {
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
        let output = Command::new("sh").arg("-c").arg(command).output()?;
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("Command failed: {}. Stderr: {}", result, error);
        }
        Ok(result)
    }
}
