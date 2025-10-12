//! MCP (Model Context Protocol) integration for the OxideAgent system.
//!
//! This module provides functionality for connecting to and managing MCP servers,
//! both remote and local, with support for various deployment methods.

use crate::tools::{Tool, ToolProfile};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Definition of an MCP tool
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpToolDefinition {
    /// Name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON schema for the tool's parameters
    #[serde(rename = "inputSchema", default)]
    pub input_schema: Value,
}

/// Adapter that wraps an MCP tool definition to implement the Tool trait
pub struct McpToolAdapter {
    definition: McpToolDefinition,
    mcp_url: String,
    auth_token: Option<String>,
}

impl McpToolAdapter {
    /// Create a new MCP tool adapter
    pub fn new(
        definition: McpToolDefinition,
        mcp_url: String,
        auth_token: Option<String>,
    ) -> Self {
        Self {
            definition,
            mcp_url,
            auth_token,
        }
    }
}

impl Tool for McpToolAdapter {
    fn name(&self) -> String {
        self.definition.name.clone()
    }

    fn description(&self) -> String {
        self.definition.description.clone()
    }
    
    fn parameters(&self) -> Value {
        // Map the input_schema to the parameters format expected by Ollama
        self.definition.input_schema.clone()
    }
    
    fn profile(&self) -> ToolProfile {
        ToolProfile::Generic
    }

    fn execute(&self, args: &Value) -> anyhow::Result<String> {
        // Here we would make an HTTP call to the MCP server
        // For now, we'll just return a placeholder
        Ok(format!(
            "Called MCP tool '{}' with args: {} at URL: {}",
            self.definition.name,
            args,
            self.mcp_url
        ))
    }
}

/// Connection to an MCP server
pub struct McpConnection {
    client: Client,
    url: String,
    auth_token: Option<String>,
}

impl McpConnection {
    /// Create a new MCP connection
    pub fn new(url: String, auth_token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            url,
            auth_token,
        }
    }

    /// Discover available tools from the MCP server (for the legacy HTTP approach)
    pub async fn discover_tools(&self) -> anyhow::Result<Vec<McpToolDefinition>> {
        let mut request = self.client.get(&format!("{}/tools", self.url));
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await?;
        let tools: Vec<McpToolDefinition> = response.json().await?;
        Ok(tools)
    }

    /// Get the URL of the MCP server
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// Get the authentication token for the MCP server
    pub fn auth_token(&self) -> Option<String> {
        self.auth_token.clone()
    }
}
