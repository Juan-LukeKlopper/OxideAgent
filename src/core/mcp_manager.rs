// MCP Manager
//
// This module is responsible for launching and managing MCP servers.

use crate::config::{MCPConfig, MCPToolConfig};
use crate::core::mcp::config::{McpServerConfig, McpServerType};
use crate::core::mcp::connection::StdioMcpConnection;
use crate::core::mcp::manager::{McpManager as NewMcpManager, McpToolAdapter};
use crate::core::tools::ToolRegistry;
use anyhow::Result;
use tracing::{error, info};

/// Truncate a description to the first 60 characters with an ellipsis if needed
fn truncate_description(description: &str) -> String {
    if description.len() > 60 {
        format!("{}...", &description[..60])
    } else {
        description.to_string()
    }
}

pub struct McpManager {
    tool_registry: ToolRegistry,
    new_manager: NewMcpManager,
}

impl McpManager {
    pub fn new(tool_registry: ToolRegistry) -> Self {
        Self {
            tool_registry,
            new_manager: NewMcpManager::new(),
        }
    }

    pub async fn launch_servers(&mut self, tools: &[MCPToolConfig]) -> Result<()> {
        info!(
            "Starting MCP server launch process for {} tools",
            tools.len()
        );

        // Use the new manager to launch servers
        self.new_manager.launch_servers(tools).await?;

        // Now we need to create tool adapters that reference the connections in the registry
        for tool_config in tools {
            // Connect to the server to discover its tools (for registration purposes)
            let server_type = McpServerType::Command {
                command: tool_config.command.clone(),
                args: Some(tool_config.args.clone()),
                environment: None,
                working_directory: None,
            };

            let config = McpServerConfig {
                name: tool_config.name.clone(),
                description: Some(format!("MCP server for {}", tool_config.name)),
                server_type,
                auto_start: Some(true),
                environment: None,
            };

            // Create a temporary connection to discover tools (this is not ideal but needed for now)
            match StdioMcpConnection::new(&config).await {
                Ok(mut connection) => {
                    match connection.discover_tools().await {
                        Ok(mcp_tools) => {
                            info!(
                                "Discovered {} tools from MCP server '{}':",
                                mcp_tools.len(),
                                tool_config.name
                            );

                            let connection_id = format!("{}_connection", tool_config.name);

                            // Add discovered tools to the registry as adapters
                            for mcp_tool in mcp_tools {
                                info!(
                                    "  - Adding MCP tool adapter: {} - {}",
                                    mcp_tool.name,
                                    truncate_description(&mcp_tool.description)
                                );

                                // Create an adapter for the tool
                                let tool_adapter = McpToolAdapter::new(
                                    mcp_tool.name.clone(),
                                    mcp_tool.description.clone(),
                                    mcp_tool.input_schema.clone(),
                                    connection_id.clone(),
                                );

                                self.tool_registry.add_tool(Box::new(tool_adapter));
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to discover tools from MCP server '{}': {}",
                                tool_config.name, e
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to establish stdio connection to MCP server '{}': {}",
                        tool_config.name, e
                    );
                }
            }
        }

        // Log all currently registered tools (including MCP tools)
        let all_tools = self.tool_registry.definitions();
        info!("Total tools available in the system: {}", all_tools.len());
        for tool in &all_tools {
            info!(
                "  - Available: {} - {}",
                tool.function.name,
                tool.truncated_description()
            );
        }

        info!("Completed MCP server launch process");
        Ok(())
    }

    pub async fn launch_remote_server(&mut self, config: &MCPConfig) -> Result<()> {
        if let Some(server_url) = &config.server {
            info!("Processing remote MCP server: {}", server_url);

            // Create a server config for the remote server
            let server_config = McpServerConfig {
                name: "remote_mcp_server".to_string(),
                description: Some(format!("Remote MCP server at {}", server_url)),
                server_type: McpServerType::Remote {
                    url: server_url.clone(),
                    access_token: config.auth_token.clone(),
                    api_key: None, // API key not currently supported in the main config
                },
                auto_start: Some(true),
                environment: None,
            };

            // Handle remote server launch using the new manager's approach
            match &server_config.server_type {
                McpServerType::Remote {
                    url,
                    access_token,
                    api_key,
                } => {
                    // For remote servers, create an HTTP connection
                    info!(
                        "Creating HTTP connection to remote MCP server at URL: {}",
                        url
                    );

                    let http_connection = crate::core::mcp::http::HttpMcpConnection::new(
                        &server_config,
                        url.clone(),
                        access_token.clone(),
                        api_key.clone(),
                    );

                    // Add connection to the registry
                    let connection_id = format!("{}_connection", server_config.name);
                    self.new_manager
                        .get_registry()
                        .add_http_connection(connection_id.clone(), http_connection)
                        .await;

                    // Discover tools from the remote server using the registry
                    match self
                        .new_manager
                        .get_registry()
                        .discover_tools_on_connection(&connection_id)
                        .await
                    {
                        Ok(mcp_tools) => {
                            info!(
                                "Discovered {} tools from HTTP MCP server '{}':",
                                mcp_tools.len(),
                                server_config.name
                            );

                            // Add discovered tools to the registry as adapters
                            for mcp_tool in mcp_tools {
                                info!(
                                    "  - Adding MCP tool adapter: {} - {}",
                                    mcp_tool.name,
                                    truncate_description(&mcp_tool.description)
                                );

                                // Create an adapter for the tool
                                let tool_adapter = McpToolAdapter::new(
                                    mcp_tool.name.clone(),
                                    mcp_tool.description.clone(),
                                    mcp_tool.input_schema.clone(),
                                    connection_id.clone(),
                                );

                                self.tool_registry.add_tool(Box::new(tool_adapter));
                            }

                            info!(
                                "HTTP MCP server '{}' is running and ready for communication at URL: {}",
                                server_config.name, url
                            );
                        }
                        Err(e) => {
                            error!(
                                "Failed to discover tools from HTTP MCP server '{}': {}",
                                server_config.name, e
                            );
                        }
                    }
                }
                _ => {
                    // This shouldn't happen for a remote server, but just to satisfy the compiler
                    error!("Unexpected server type in remote server processing");
                }
            }
        }
        Ok(())
    }

    pub fn into_tool_registry(self) -> ToolRegistry {
        self.tool_registry
    }
}
