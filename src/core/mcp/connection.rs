//! Implementation of the Model Context Protocol (MCP) for stdio-based communication.
//!
//! This module implements the official MCP specification for communication
//! between agents and MCP-compatible tools/services over stdio.

use crate::core::mcp::config::McpServerConfig;
use crate::core::tools::{Tool, ToolProfile};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info};

/// Truncate a description to the first 60 characters with an ellipsis if needed
fn truncate_description(description: &str) -> String {
    if description.len() > 60 {
        format!("{}...", &description[..60])
    } else {
        description.to_string()
    }
}

/// Common trait for all MCP connections
#[async_trait::async_trait]
pub trait McpConnection: Send + Sync {
    /// Discover available tools from the MCP server
    async fn discover_tools(&mut self) -> Result<Vec<McpToolDefinition>>;

    /// Execute a tool on the MCP server
    async fn execute_tool(&mut self, tool_name: &str, args: &Value) -> Result<Value>;
}

/// MCP tool definition as specified in the MCP specification
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpToolDefinition {
    /// Name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON schema for the tool's parameters (may be optional in some implementations)
    #[serde(rename = "inputSchema", default)]
    pub input_schema: Value,
}

/// JSON-RPC 2.0 request structure
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u32,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC 2.0 success response structure
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcSuccessResponse {
    jsonrpc: String,
    id: u32,
    result: Value,
}

/// JSON-RPC 2.0 error response structure
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcErrorResponse {
    jsonrpc: String,
    id: u32,
    error: JsonRpcErrorObject,
}

/// JSON-RPC 2.0 error object
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcErrorObject {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Response structure for tools/list method
#[derive(Serialize, Deserialize, Debug)]
struct ToolsListResult {
    tools: Vec<McpToolDefinition>,
}

/// MCP connection that communicates over stdio using JSON-RPC 2.0
#[derive(Debug)]
pub struct StdioMcpConnection {
    /// Sender for writing messages to the MCP server's stdin
    stdin_tx: mpsc::UnboundedSender<String>,
    /// Receiver for reading messages from the MCP server's stdout
    stdout_rx: mpsc::UnboundedReceiver<String>,
    /// Counter for generating unique request IDs
    request_id_counter: u32,
    /// Name of the server for logging purposes
    server_name: String,
}

impl StdioMcpConnection {
    /// Create a new stdio connection to an MCP server by launching it as a subprocess
    pub async fn new(config: &McpServerConfig) -> Result<Self> {
        info!("Launching MCP server '{}' via stdio", config.name);

        // Create the command based on the server configuration
        let mut command = match &config.server_type {
            crate::core::mcp::config::McpServerType::Command {
                command,
                args,
                working_directory,
                environment,
            } => {
                let mut cmd = tokio::process::Command::new(command);

                if let Some(args_list) = args {
                    cmd.args(args_list);
                }

                if let Some(dir) = working_directory {
                    cmd.current_dir(dir);
                }

                if let Some(env_vars) = environment {
                    for (key, value) in env_vars {
                        cmd.env(key, value);
                    }
                }

                cmd
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Only command-based MCP servers are supported via stdio, got: {:?}",
                    config.server_type
                ));
            }
        };

        // Launch the process with stdio pipes
        let mut child = command
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()?;

        let process_id = child.id();
        info!(
            "MCP server '{}' launched with PID: {:?}",
            config.name, process_id
        );

        // Get the stdio handles
        let stdin = child.stdin.take().expect("Failed to get stdin handle");
        let stdout = child.stdout.take().expect("Failed to get stdout handle");

        // Create channels for communication
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();
        let (stdout_tx, stdout_rx) = mpsc::unbounded_channel::<String>();

        // Spawn task to handle writing to stdin
        let mut stdin_handle = stdin;
        tokio::spawn(async move {
            while let Some(message) = stdin_rx.recv().await {
                debug!("Sending to MCP server: {}", message);
                if let Err(e) = stdin_handle.write_all(message.as_bytes()).await {
                    error!("Error writing to MCP server stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin_handle.write_all(b"\n").await {
                    error!("Error writing newline to MCP server stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin_handle.flush().await {
                    error!("Error flushing MCP server stdin: {}", e);
                    break;
                }
            }
        });

        // Spawn task to handle reading from stdout
        let mut reader = BufReader::new(stdout).lines();
        let stdout_sender = stdout_tx.clone();
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                if !line.trim().is_empty() {
                    debug!("Received from MCP server: {}", line);
                    if stdout_sender.send(line.trim().to_string()).is_err() {
                        // Receiver dropped
                        break;
                    }
                }
            }
        });

        info!(
            "Established stdio connection to MCP server '{}'",
            config.name
        );

        Ok(Self {
            stdin_tx,
            stdout_rx,
            request_id_counter: 1,
            server_name: config.name.clone(),
        })
    }

    /// Send a JSON-RPC request to the MCP server and wait for a response
    pub async fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let request_id = self.request_id_counter;
        self.request_id_counter += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        debug!("Sending JSON-RPC request: {}", request_json);

        // Send the request
        if self.stdin_tx.send(request_json).is_err() {
            return Err(anyhow::anyhow!("Failed to send request to MCP server"));
        }

        // Wait for response with timeout
        match timeout(Duration::from_secs(10), self.stdout_rx.recv()).await {
            Ok(Some(response_str)) => {
                debug!("Received JSON-RPC response: {}", response_str);

                // Try to parse as success response first
                if let Ok(success_response) =
                    serde_json::from_str::<JsonRpcSuccessResponse>(&response_str)
                {
                    if success_response.id == request_id {
                        Ok(success_response.result)
                    } else {
                        Err(anyhow::anyhow!("Response ID mismatch"))
                    }
                }
                // Try to parse as error response
                else if let Ok(error_response) =
                    serde_json::from_str::<JsonRpcErrorResponse>(&response_str)
                {
                    if error_response.id == request_id {
                        Err(anyhow::anyhow!(
                            "MCP server error {}: {}",
                            error_response.error.code,
                            error_response.error.message
                        ))
                    } else {
                        Err(anyhow::anyhow!("Response ID mismatch"))
                    }
                }
                // Failed to parse as either type
                else {
                    Err(anyhow::anyhow!(
                        "Failed to parse MCP server response: {}",
                        response_str
                    ))
                }
            }
            Ok(None) => Err(anyhow::anyhow!("MCP server closed connection")),
            Err(_) => Err(anyhow::anyhow!(
                "Timeout waiting for response from MCP server"
            )),
        }
    }

    /// Discover available tools from the MCP server using the tools/list method
    pub async fn discover_tools(&mut self) -> Result<Vec<McpToolDefinition>> {
        info!("Discovering tools from MCP server '{}'", self.server_name);

        match self.send_request("tools/list", None).await {
            Ok(result) => match serde_json::from_value::<ToolsListResult>(result) {
                Ok(tools_list) => {
                    info!(
                        "Successfully discovered {} tools from MCP server '{}'",
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
                }
                Err(e) => {
                    error!(
                        "Failed to parse tools list response from MCP server '{}': {}",
                        self.server_name, e
                    );
                    Err(anyhow::anyhow!(
                        "Failed to parse tools list response: {}",
                        e
                    ))
                }
            },
            Err(e) => {
                error!(
                    "Failed to discover tools from MCP server '{}': {}",
                    self.server_name, e
                );
                Err(e)
            }
        }
    }
}

#[async_trait::async_trait]
impl McpConnection for StdioMcpConnection {
    async fn discover_tools(&mut self) -> Result<Vec<McpToolDefinition>> {
        self.discover_tools().await // Call the public method
    }

    async fn execute_tool(&mut self, tool_name: &str, args: &Value) -> Result<Value> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": args
        });

        // Use the existing send_request method
        self.send_request("tools/call", Some(params)).await
    }
}

// NOTE: The StdioMcpToolAdapter is kept for potential future use
// but is not currently used since we now use McpToolAdapter from the manager module
#[allow(dead_code)]
/// Adapter that implements the Tool trait for MCP tools
pub struct StdioMcpToolAdapter {
    name: String,
    description: String,
    parameters: Value,
    // NOTE: This is a placeholder implementation since the connection
    // would need to be maintained outside of the adapter.
    // In a complete implementation, this would store a reference or ID
    // to allow the system to route tool calls to the correct connection.
}

#[allow(dead_code)]
impl StdioMcpToolAdapter {
    /// Create a new MCP tool adapter
    pub fn new(name: String, description: String, parameters: Value) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

#[async_trait::async_trait]
impl Tool for StdioMcpToolAdapter {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    fn profile(&self) -> ToolProfile {
        ToolProfile::Generic
    }

    async fn execute(&self, _args: &Value) -> anyhow::Result<String> {
        // In a complete implementation, this would need to communicate with the actual MCP server
        // This is a placeholder that indicates the functionality is not fully implemented
        Err(anyhow::anyhow!(
            "MCP tool execution is not fully implemented yet. Tool '{}' would be executed with provided args.",
            self.name
        ))
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(StdioMcpToolAdapter {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_mcp_tool_definition_creation() {
        let tool_def = McpToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string"
                    }
                },
                "required": ["param1"]
            }),
        };

        assert_eq!(tool_def.name, "test_tool");
        assert_eq!(tool_def.description, "A test tool");
        assert!(!tool_def.input_schema.is_null());
    }

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
    async fn test_json_rpc_request_structure() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test_method".to_string(),
            params: Some(serde_json::json!({"test": "value"})),
        };

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, 1);
        assert_eq!(request.method, "test_method");
        assert!(request.params.is_some());
    }

    #[tokio::test]
    async fn test_json_rpc_success_response_structure() {
        let response = JsonRpcSuccessResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: serde_json::json!("success"),
        };

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert_eq!(response.result, serde_json::json!("success"));
    }

    #[tokio::test]
    async fn test_json_rpc_error_response_structure() {
        let response = JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            error: JsonRpcErrorObject {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: Some(serde_json::json!("error details")),
            },
        };

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert_eq!(response.error.code, -32600);
        assert_eq!(response.error.message, "Invalid Request");
        assert!(response.error.data.is_some());
    }

    #[tokio::test]
    async fn test_tools_list_result_structure() {
        let tool_def = McpToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string"
                    }
                },
                "required": ["param1"]
            }),
        };

        let result = ToolsListResult {
            tools: vec![tool_def],
        };

        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_mcp_connection_trait_implementation() {
        use async_trait::async_trait;

        struct MockConnection {
            counter: u32,
        }

        #[async_trait]
        impl McpConnection for MockConnection {
            async fn discover_tools(&mut self) -> Result<Vec<McpToolDefinition>> {
                self.counter += 1;
                Ok(vec![])
            }

            async fn execute_tool(&mut self, _tool_name: &str, _args: &Value) -> Result<Value> {
                self.counter += 1;
                Ok(serde_json::json!("executed"))
            }
        }

        let mut conn = MockConnection { counter: 0 };
        let _ = conn.discover_tools().await;
        let _ = conn.execute_tool("test", &serde_json::json!({})).await;
        assert_eq!(conn.counter, 2);
    }

    #[tokio::test]
    async fn test_stdio_mcp_tool_adapter_creation() {
        let adapter = StdioMcpToolAdapter {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({}),
        };

        assert_eq!(adapter.name(), "test_tool");
        assert_eq!(adapter.description(), "A test tool");
        assert_eq!(adapter.profile(), ToolProfile::Generic);
    }

    #[tokio::test]
    async fn test_stdio_mcp_tool_adapter_clone() {
        let adapter = StdioMcpToolAdapter {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({}),
        };

        let cloned_adapter = adapter.clone_box();
        assert_eq!(cloned_adapter.name(), "test_tool");
        assert_eq!(cloned_adapter.description(), "A test tool");
    }

    #[tokio::test]
    async fn test_stdio_mcp_connection_creation_with_non_command_type() {
        use crate::core::mcp::config::McpServerType;

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

        // This should return an error because only command-based servers are supported via stdio
        let result = StdioMcpConnection::new(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_truncate_description_function() {
        // Test short description
        let short_desc = "This is a short description";
        let result = truncate_description(short_desc);
        assert_eq!(result, short_desc);

        // Test long description that should be truncated
        let long_desc = "This is a very long description that definitely exceeds sixty characters and should be truncated with an ellipsis at the end";
        let result = truncate_description(long_desc);
        assert_eq!(result.len(), 63); // 60 + 3 for "..."
        assert!(result.ends_with("..."));
    }

    #[tokio::test]
    async fn test_mcp_tool_definition_structure() {
        let tool_def = McpToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool for testing purposes".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string"
                    }
                },
                "required": ["param1"]
            }),
        };

        assert_eq!(tool_def.name, "test_tool");
        assert_eq!(tool_def.description, "A test tool for testing purposes");
        assert!(!tool_def.input_schema.is_null());
    }

    #[tokio::test]
    async fn test_json_rpc_structures() {
        // Test JsonRpcRequest
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test_method".to_string(),
            params: Some(serde_json::json!({"test": "value"})),
        };

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, 1);
        assert_eq!(request.method, "test_method");
        assert!(request.params.is_some());

        // Test JsonRpcSuccessResponse
        let success_response = JsonRpcSuccessResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: serde_json::json!("test_result"),
        };

        assert_eq!(success_response.jsonrpc, "2.0");
        assert_eq!(success_response.id, 1);
        assert_eq!(success_response.result, serde_json::json!("test_result"));

        // Test JsonRpcErrorResponse
        let error_response = JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            error: JsonRpcErrorObject {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: Some(serde_json::json!({"details": "error details"})),
            },
        };

        assert_eq!(error_response.jsonrpc, "2.0");
        assert_eq!(error_response.id, 1);
        assert_eq!(error_response.error.code, -32600);
        assert_eq!(error_response.error.message, "Invalid Request");
        assert!(error_response.error.data.is_some());

        // Test ToolsListResult
        let tool_def = McpToolDefinition {
            name: "listed_tool".to_string(),
            description: "A tool in the list".to_string(),
            input_schema: serde_json::json!({}),
        };

        let tools_list = ToolsListResult {
            tools: vec![tool_def],
        };

        assert_eq!(tools_list.tools.len(), 1);
        assert_eq!(tools_list.tools[0].name, "listed_tool");
    }

    // NOTE: The following tests would require launching actual processes
    // which is not feasible in a unit test environment. These methods are
    // extensively tested through integration tests and manual testing.
    // Mock-based testing would require significant refactoring of the
    // StdioMcpConnection to allow dependency injection of the process
    // communication layer.
}
