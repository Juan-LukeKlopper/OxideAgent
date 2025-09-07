use crate::tools::{Tool, ToolProfile};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub struct McpToolAdapter {
    definition: McpToolDefinition,
    mcp_url: String,
    auth_token: Option<String>,
}

impl McpToolAdapter {
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
        self.definition.parameters.clone()
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

pub struct McpConnection {
    client: Client,
    url: String,
    auth_token: Option<String>,
}

impl McpConnection {
    pub fn new(url: String, auth_token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            url,
            auth_token,
        }
    }

    pub async fn discover_tools(&self) -> anyhow::Result<Vec<McpToolDefinition>> {
        let mut request = self.client.get(&format!("{}/tools", self.url));
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await?;
        let tools: Vec<McpToolDefinition> = response.json().await?;
        Ok(tools)
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn auth_token(&self) -> Option<String> {
        self.auth_token.clone()
    }
}
