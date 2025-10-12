//! Implementation of the Model Context Protocol (MCP) for HTTP-based communication.
//!
//! This module implements the official MCP specification for communication
//! between agents and MCP-compatible tools/services over HTTP.

use crate::core::mcp::config::McpServerConfig;
use crate::core::mcp::connection::{McpConnection, McpToolDefinition};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, warn};

/// MCP connection that communicates over HTTP using JSON-RPC 2.0
pub struct HttpMcpConnection {
    /// HTTP client for making requests
    client: Client,
    /// Base URL of the MCP server
    base_url: String,
    /// Access token for authentication (if provided)
    access_token: Option<String>,
    /// API key for authentication (if provided)
    api_key: Option<String>,
    /// Name of the server for logging purposes
    server_name: String,
}

impl HttpMcpConnection {
    /// Create a new HTTP connection to an MCP server
    pub fn new(
        config: &McpServerConfig,
        url: String,
        access_token: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url: url,
            access_token,
            api_key,
            server_name: config.name.clone(),
        }
    }
}

#[async_trait::async_trait]
impl McpConnection for HttpMcpConnection {
    /// Discover available tools from the MCP server using the tools/list method
    async fn discover_tools(&mut self) -> Result<Vec<McpToolDefinition>> {
        info!(
            "Discovering tools from HTTP MCP server '{}'",
            self.server_name
        );

        // First, attempt the MCP initialization handshake as per spec using JSON-RPC to the same URL
        if let Err(init_err) = self.initialize_connection().await {
            warn!(
                "MCP initialization handshake failed for server '{}': {}. Proceeding with tools discovery anyway.",
                self.server_name, init_err
            );
        }

        // Use the base URL directly as provided by the configuration - don't append anything
        let url = &self.base_url;

        // JSON-RPC request for tools (sent to the same URL as initialization)
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });

        let mut request_builder = self.client.post(url).json(&request_body);

        // Add authentication headers if provided
        if let Some(token) = &self.access_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
        }

        if let Some(key) = &self.api_key {
            request_builder = request_builder.header("X-API-Key", key);
        }

        // Add the required JSON-RPC content type and accept headers
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        match request_builder.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() || status.as_u16() == 200 {
                    // Try to get the response body as text first for debugging
                    let response_text = match response.text().await {
                        Ok(text) => {
                            debug!("Raw tools discovery response body: {}", text);
                            text
                        }
                        Err(e) => {
                            error!(
                                "Failed to read tools discovery response body as text: {}",
                                e
                            );
                            return Err(anyhow::anyhow!(
                                "Failed to read response body as text: {}",
                                e
                            ));
                        }
                    };

                    // Handle SSE format response
                    let json_data = if response_text.starts_with("event:") {
                        // Extract JSON from SSE format
                        if let Some(data_line) =
                            response_text.lines().find(|line| line.starts_with("data:"))
                        {
                            data_line.strip_prefix("data:").unwrap_or("").trim()
                        } else {
                            &response_text
                        }
                    } else {
                        &response_text
                    };

                    // Try to parse as JSON
                    match serde_json::from_str::<serde_json::Value>(json_data) {
                        Ok(json_response) => {
                            debug!("Parsed JSON response: {:?}", json_response);

                            // Check if there's an error in the response
                            if let Some(error) = json_response.get("error") {
                                error!(
                                    "MCP server '{}' returned error during tools discovery: {:?} (raw response: {})",
                                    self.server_name, error, response_text
                                );
                                return Err(anyhow::anyhow!(
                                    "MCP server returned error: {:?}",
                                    error
                                ));
                            }

                            // Parse the tools list from the response
                            if let Some(result) = json_response.get("result") {
                                if let Ok(tools_list) =
                                    serde_json::from_value::<ToolsListResult>(result.clone())
                                {
                                    info!(
                                        "Successfully discovered {} tools from HTTP MCP server '{}'",
                                        tools_list.tools.len(),
                                        self.server_name
                                    );

                                    for tool in &tools_list.tools {
                                        info!(
                                            "  - Tool: {} - {}",
                                            tool.name,
                                            truncate_description(&tool.description)
                                        );
                                    }

                                    Ok(tools_list.tools)
                                } else {
                                    error!(
                                        "Failed to parse tools list response from HTTP MCP server '{}' (raw response: {})",
                                        self.server_name, response_text
                                    );
                                    Err(anyhow::anyhow!("Failed to parse tools list response"))
                                }
                            } else {
                                error!(
                                    "No result field in response from HTTP MCP server '{}' (raw response: {})",
                                    self.server_name, response_text
                                );
                                Err(anyhow::anyhow!("No result field in response"))
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to parse JSON response from HTTP MCP server '{}': {} (raw response: {})",
                                self.server_name, e, response_text
                            );
                            Err(anyhow::anyhow!(
                                "Failed to parse JSON response: {} (raw response: {})",
                                e,
                                response_text
                            ))
                        }
                    }
                } else {
                    // Try to get more detailed error information
                    let error_text = match response.text().await {
                        Ok(text) => format!(" (response body: {})", text),
                        Err(_) => String::new(),
                    };
                    error!(
                        "JSON-RPC request to MCP server '{}' failed with status: {}{}",
                        self.server_name, status, error_text
                    );
                    Err(anyhow::anyhow!(
                        "JSON-RPC request failed with status: {}{}",
                        status,
                        error_text
                    ))
                }
            }
            Err(e) => {
                error!(
                    "Failed to send JSON-RPC request to MCP server '{}': {}",
                    self.server_name, e
                );

                Err(anyhow::anyhow!("JSON-RPC request failed: {}", e))
            }
        }
    }

    /// Execute a tool on the MCP server
    async fn execute_tool(&mut self, tool_name: &str, args: &Value) -> Result<Value> {
        info!(
            "Executing tool '{}' on HTTP MCP server '{}'",
            tool_name, self.server_name
        );

        // Use the base URL directly as provided by the configuration - don't append anything
        let url = &self.base_url;
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": args
            }
        });

        let mut request_builder = self.client.post(url).json(&request_body);

        // Add authentication headers if provided
        if let Some(token) = &self.access_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
        }

        if let Some(key) = &self.api_key {
            request_builder = request_builder.header("X-API-Key", key);
        }

        // Add the required JSON-RPC content type and accept headers
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        match request_builder.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    // Try to get the response body as text first for debugging
                    let response_text = match response.text().await {
                        Ok(text) => {
                            debug!("Raw tool execution response body: {}", text);
                            text
                        }
                        Err(e) => {
                            error!("Failed to read tool execution response body as text: {}", e);
                            return Err(anyhow::anyhow!(
                                "Failed to read response body as text: {}",
                                e
                            ));
                        }
                    };

                    // Handle SSE format response
                    let json_data = if response_text.starts_with("event:") {
                        // Extract JSON from SSE format
                        if let Some(data_line) =
                            response_text.lines().find(|line| line.starts_with("data:"))
                        {
                            data_line.strip_prefix("data:").unwrap_or("").trim()
                        } else {
                            &response_text
                        }
                    } else {
                        &response_text
                    };

                    // Try to parse as JSON
                    match serde_json::from_str::<serde_json::Value>(json_data) {
                        Ok(json_response) => {
                            debug!("Received HTTP response: {:?}", json_response);

                            // Check if there's an error in the response
                            if let Some(error) = json_response.get("error") {
                                error!(
                                    "MCP server '{}' returned error during tool execution '{}': {:?} (raw response: {})",
                                    self.server_name, tool_name, error, response_text
                                );
                                return Err(anyhow::anyhow!(
                                    "MCP server returned error: {:?}",
                                    error
                                ));
                            }

                            // Return the result from the response
                            if let Some(result) = json_response.get("result") {
                                info!(
                                    "Successfully executed tool '{}' on HTTP MCP server '{}'",
                                    tool_name, self.server_name
                                );
                                Ok(result.clone())
                            } else {
                                error!(
                                    "No result field in response from HTTP MCP server '{}' for tool '{}' (raw response: {})",
                                    self.server_name, tool_name, response_text
                                );
                                Err(anyhow::anyhow!("No result field in response"))
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to parse JSON response from HTTP MCP server '{}' for tool '{}': {} (raw response: {})",
                                self.server_name, tool_name, e, response_text
                            );
                            Err(anyhow::anyhow!(
                                "Failed to parse JSON response: {} (raw response: {})",
                                e,
                                response_text
                            ))
                        }
                    }
                } else {
                    // Try to get more detailed error information
                    let error_text = match response.text().await {
                        Ok(text) => format!(" (response body: {})", text),
                        Err(_) => String::new(),
                    };
                    error!(
                        "JSON-RPC request to execute tool '{}' on MCP server '{}' failed with status: {}{}",
                        tool_name, self.server_name, status, error_text
                    );
                    Err(anyhow::anyhow!(
                        "JSON-RPC request failed with status: {}{}",
                        status,
                        error_text
                    ))
                }
            }
            Err(e) => {
                error!(
                    "Failed to send JSON-RPC request to execute tool '{}' on MCP server '{}': {}",
                    tool_name, self.server_name, e
                );
                Err(anyhow::anyhow!("JSON-RPC request failed: {}", e))
            }
        }
    }
}

impl HttpMcpConnection {
    /// Initialize the MCP connection following the MCP specification using JSON-RPC
    async fn initialize_connection(&mut self) -> Result<()> {
        info!(
            "Initializing MCP connection for server '{}'",
            self.server_name
        );

        // Use the same base URL for initialization as per MCP spec
        // All MCP communication should happen on the same endpoint
        let url = &self.base_url;

        // Prepare the initialize request body according to MCP spec
        // This is a JSON-RPC request to the same URL
        let init_request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2024-11-05", // Use a supported protocol version
                "clientInfo": {
                    "name": "OxideAgent",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    },
                    "resources": {},
                    "prompts": {},
                    "logging": {}
                }
            }
        });

        let mut init_request_builder = self.client.post(url).json(&init_request_body);

        // Add authentication headers if provided
        if let Some(token) = &self.access_token {
            init_request_builder =
                init_request_builder.header("Authorization", format!("Bearer {}", token));
        }

        if let Some(key) = &self.api_key {
            init_request_builder = init_request_builder.header("X-API-Key", key);
        }

        // Add content type and required accept headers
        init_request_builder = init_request_builder
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        match init_request_builder.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    // Try to get the response body as text first for debugging
                    let response_text = match response.text().await {
                        Ok(text) => {
                            debug!("Raw initialize response body: {}", text);
                            text
                        }
                        Err(e) => {
                            warn!("Failed to read initialize response body as text: {}", e);
                            return Ok(()); // Continue anyway
                        }
                    };

                    // Handle SSE format response
                    let json_data = if response_text.starts_with("event:") {
                        // Extract JSON from SSE format
                        if let Some(data_line) =
                            response_text.lines().find(|line| line.starts_with("data:"))
                        {
                            data_line.strip_prefix("data:").unwrap_or("").trim()
                        } else {
                            &response_text
                        }
                    } else {
                        &response_text
                    };

                    // Try to parse as JSON
                    match serde_json::from_str::<serde_json::Value>(json_data) {
                        Ok(init_response) => {
                            // Check if there's an error in the response
                            if let Some(error) = init_response.get("error") {
                                warn!(
                                    "MCP server '{}' returned error during initialization: {:?}",
                                    self.server_name, error
                                );
                                // Don't treat this as fatal error, just warn the user
                                Ok(())
                            } else {
                                info!("MCP server '{}' initialized successfully", self.server_name);
                                debug!("Initialize response: {:?}", init_response);

                                // After successful initialization, send the initialized notification
                                if let Err(notif_err) = self.send_initialized_notification().await {
                                    warn!("Failed to send initialized notification: {}", notif_err);
                                    // Don't treat this as fatal, just continue
                                }

                                Ok(())
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to parse initialize response from MCP server '{}' as JSON: {} (raw response: {})",
                                self.server_name, e, response_text
                            );
                            // Don't treat this as fatal error, just warn the user
                            Ok(())
                        }
                    }
                } else {
                    // Try to get more detailed error information
                    let error_text = match response.text().await {
                        Ok(text) => format!(" (response body: {})", text),
                        Err(_) => String::new(),
                    };
                    warn!(
                        "Initialize request to MCP server '{}' failed with status: {}{}",
                        self.server_name, status, error_text
                    );
                    Ok(()) // Don't treat this as fatal error to maintain compatibility
                }
            }
            Err(e) => {
                warn!(
                    "Failed to send initialize request to MCP server '{}': {}",
                    self.server_name, e
                );
                Ok(()) // Don't treat this as fatal error to maintain compatibility
            }
        }
    }

    /// Send the initialized notification after successful initialization
    async fn send_initialized_notification(&self) -> Result<()> {
        info!(
            "Sending initialized notification for server '{}'",
            self.server_name
        );

        // Use the same base URL as specified in the MCP spec
        let url = &self.base_url;

        // Prepare the initialized notification body as per MCP spec
        // This is a JSON-RPC notification (no id field)
        let notification_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let mut notification_builder = self.client.post(url).json(&notification_body);

        // Add authentication headers if provided
        if let Some(token) = &self.access_token {
            notification_builder =
                notification_builder.header("Authorization", format!("Bearer {}", token));
        }

        if let Some(key) = &self.api_key {
            notification_builder = notification_builder.header("X-API-Key", key);
        }

        // Add content type and required accept headers
        notification_builder = notification_builder
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        match notification_builder.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    info!(
                        "Initialized notification sent successfully to server '{}'",
                        self.server_name
                    );
                    Ok(())
                } else {
                    warn!(
                        "Initialized notification to MCP server '{}' failed with status: {}",
                        self.server_name, status
                    );
                    Ok(()) // Don't treat this as fatal error to maintain compatibility
                }
            }
            Err(e) => {
                warn!(
                    "Failed to send initialized notification to MCP server '{}': {}",
                    self.server_name, e
                );
                Ok(()) // Don't treat this as fatal error to maintain compatibility
            }
        }
    }
}

/// Response structure for tools/list method
#[derive(Serialize, Deserialize, Debug)]
struct ToolsListResult {
    tools: Vec<McpToolDefinition>,
}

/// Truncate a description to the first 60 characters with an ellipsis if needed
fn truncate_description(description: &str) -> String {
    if description.len() > 60 {
        format!("{}...", &description[..60])
    } else {
        description.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mcp::config::McpServerType;
    use tokio;

    #[tokio::test]
    async fn test_truncate_description_short() {
        let desc = "Short description";
        let result = truncate_description(desc);
        assert_eq!(result, "Short description");
    }

    #[tokio::test]
    async fn test_truncate_description_long() {
        let desc = "This is a very long description that exceeds sixty characters and should be truncated with an ellipsis";
        let result = truncate_description(desc);
        assert!(result.len() <= 63); // 60 + 3 for "..."
        assert!(result.ends_with("..."));
    }

    #[tokio::test]
    async fn test_http_mcp_connection_creation() {
        let config = McpServerConfig {
            name: "test".to_string(),
            description: Some("test".to_string()),
            server_type: McpServerType::Remote {
                url: "http://localhost:8080".to_string(),
                access_token: None,
                api_key: None,
            },
            auto_start: Some(false),
            environment: None,
        };

        let http_conn =
            HttpMcpConnection::new(&config, "http://localhost:8080".to_string(), None, None);

        // Just test that the connection can be created
        assert_eq!(http_conn.server_name, "test");
        assert_eq!(http_conn.base_url, "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_tools_list_result_structure() {
        let tool_def = McpToolDefinition {
            name: "http_test_tool".to_string(),
            description: "A test tool for HTTP testing".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string"
                    }
                }
            }),
        };

        let tools_list = ToolsListResult {
            tools: vec![tool_def],
        };

        assert_eq!(tools_list.tools.len(), 1);
        assert_eq!(tools_list.tools[0].name, "http_test_tool");
        assert_eq!(
            tools_list.tools[0].description,
            "A test tool for HTTP testing"
        );
    }
}
