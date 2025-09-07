use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum ToolApprovalResponse {
    Allow,
    AlwaysAllow,
    AlwaysAllowSession,
    Deny,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    UserInput(String),
    ToolApproval(ToolApprovalResponse),
    AgentMessage(String),
    AgentStreamChunk(String),
    AgentStreamEnd,
    ToolRequest(Vec<ToolCall>),
    ToolResult(String, String),
    Error(String),
    SwitchSession(String), // New event for switching sessions
    SwitchAgent(String), // New event for switching agents
    ListSessions, // New event for listing sessions
    RefreshSessions, // New event for refreshing sessions without displaying response
    SessionList(Vec<String>), // New event to send session list to TUI
    SessionSwitched(String), // New event to notify TUI that session has been switched
    SessionHistory(Vec<ChatMessage>), // New event to send session history to TUI
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn tool_call(content: &str, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_calls: Some(tool_calls),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: ToolFunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl Tool {
    pub fn new(name: &str, description: &str, parameters: Value) -> Self {
        Self {
            r#type: "function".to_string(),
            function: ToolFunctionDefinition {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
            },
        }
    }
}
