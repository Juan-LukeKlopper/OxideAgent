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
    pub name: String,
    pub description: String,
    pub parameters: Value,
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
