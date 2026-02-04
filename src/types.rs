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
#[allow(dead_code)] // Variants are used in the application and form part of the public API
pub enum AppEvent {
    UserInput(String),
    ToolApproval(ToolApprovalResponse),
    AgentMessage(String),
    AgentStreamChunk(String),
    AgentStreamEnd,
    ToolRequest(Vec<ToolCall>),
    ToolResult(String, String),
    Error(String),
    SwitchSession(String),             // New event for switching sessions
    SwitchAgent(String, String), // New event for switching agents (agent_name, session_context)
    SwitchModel(String),         // New event for switching models
    ListSessions,                // New event for listing sessions
    RefreshSessions,             // New event for refreshing sessions without displaying response
    SessionList(Vec<String>),    // New event to send session list to TUI
    SessionSwitched(String),     // New event to notify TUI that session has been switched
    SessionHistory(Vec<ChatMessage>), // New event to send session history to TUI
    ContinueConversation,        // New event to continue conversation after tool execution
    AgentStatusUpdate(String, String), // New event to update agent status (agent_name, status)
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

    /// Truncate the tool description to the first 60 characters with an ellipsis if needed
    pub fn truncated_description(&self) -> String {
        if self.function.description.len() > 60 {
            format!("{}...", &self.function.description[..60])
        } else {
            self.function.description.clone()
        }
    }
}
