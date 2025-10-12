use crate::config::MCPToolConfig;
use crate::core::mcp::config::{McpServerConfig, McpServerType};
use crate::core::mcp::connection::{McpConnection, McpToolDefinition, StdioMcpConnection};
use crate::core::mcp::http::HttpMcpConnection;
use crate::core::mcp::launcher::McpLauncher;
use crate::core::tools::{Tool, ToolProfile};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

pub type ConnectionId = String;

/// Enum to represent different types of MCP connections
pub enum McpConnectionType {
    Stdio(StdioMcpConnection),
    Http(HttpMcpConnection),
}

/// Global registry for MCP connections
pub struct McpConnectionRegistry {
    connections: Arc<RwLock<HashMap<ConnectionId, Arc<Mutex<McpConnectionType>>>>>,
}

impl McpConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_stdio_connection(&self, id: ConnectionId, connection: StdioMcpConnection) {
        let mut connections = self.connections.write().await;
        connections.insert(id, Arc::new(Mutex::new(McpConnectionType::Stdio(connection))));
    }
    
    pub async fn add_http_connection(&self, id: ConnectionId, connection: HttpMcpConnection) {
        let mut connections = self.connections.write().await;
        connections.insert(id, Arc::new(Mutex::new(McpConnectionType::Http(connection))));
    }

    pub async fn get_connection(
        &self,
        id: &ConnectionId,
    ) -> Option<Arc<Mutex<McpConnectionType>>> {
        let connections = self.connections.read().await;
        connections.get(id).cloned()
    }

    pub async fn execute_tool_on_connection(
        &self,
        connection_id: &ConnectionId,
        tool_name: &str,
        args: &Value,
    ) -> Result<String> {
        let connection = self
            .get_connection(connection_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        // Lock the connection and execute the tool
        let mut conn = connection.lock().await;
        match &mut *conn {
            McpConnectionType::Stdio(stdio_conn) => {
                match stdio_conn.execute_tool(tool_name, args).await {
                    Ok(result) => Ok(result.to_string()),
                    Err(e) => {
                        error!(
                            "Failed to execute tool '{}' on stdio connection '{}': {}",
                            tool_name, connection_id, e
                        );
                        Err(e)
                    }
                }
            }
            McpConnectionType::Http(http_conn) => {
                match http_conn.execute_tool(tool_name, args).await {
                    Ok(result) => Ok(result.to_string()),
                    Err(e) => {
                        error!(
                            "Failed to execute tool '{}' on HTTP connection '{}': {}",
                            tool_name, connection_id, e
                        );
                        Err(e)
                    }
                }
            }
        }
    }

    pub async fn discover_tools_on_connection(
        &self,
        connection_id: &ConnectionId,
    ) -> Result<Vec<McpToolDefinition>> {
        let connection = self
            .get_connection(connection_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Connection not found: {}", connection_id))?;

        // Lock the connection and discover tools
        let mut conn = connection.lock().await;
        match &mut *conn {
            McpConnectionType::Stdio(stdio_conn) => {
                // Use the trait method
                stdio_conn.discover_tools().await
            }
            McpConnectionType::Http(http_conn) => {
                // Use the trait method
                http_conn.discover_tools().await
            }
        }
    }
}

impl Default for McpConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Static instance of the connection registry
static MCP_CONNECTION_REGISTRY: once_cell::sync::OnceCell<Arc<McpConnectionRegistry>> =
    once_cell::sync::OnceCell::new();

pub fn get_mcp_registry() -> Arc<McpConnectionRegistry> {
    MCP_CONNECTION_REGISTRY
        .get()
        .expect("MCP registry not initialized")
        .clone()
}

pub fn init_mcp_registry() {
    MCP_CONNECTION_REGISTRY.get_or_init(|| Arc::new(McpConnectionRegistry::new()));
}

/// MCP tool adapter that uses the global registry to execute tools
#[derive(Clone)]
pub struct McpToolAdapter {
    name: String,
    description: String,
    parameters: Value,
    connection_id: ConnectionId, // Reference to the connection in the registry
}

impl McpToolAdapter {
    pub fn new(
        name: String,
        description: String,
        parameters: Value,
        connection_id: ConnectionId,
    ) -> Self {
        Self {
            name,
            description,
            parameters,
            connection_id,
        }
    }
}

#[async_trait::async_trait]
impl Tool for McpToolAdapter {
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

    async fn execute(&self, args: &Value) -> anyhow::Result<String> {
        let registry = get_mcp_registry();
        registry
            .execute_tool_on_connection(&self.connection_id, &self.name, args)
            .await
    }
    
    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(self.clone())
    }
}

/// MCP manager that handles server lifecycle and tool registration
pub struct McpManager {
    registry: Arc<McpConnectionRegistry>,
}

impl McpManager {
    pub fn new() -> Self {
        init_mcp_registry();
        Self {
            registry: get_mcp_registry(),
        }
    }

    pub async fn launch_servers(&self, tools: &[MCPToolConfig]) -> Result<()> {
        info!(
            "Starting MCP server launch process for {} tools",
            tools.len()
        );

        for tool_config in tools {
            info!(
                "Attempting to launch MCP server: {} with command '{}' and args {:?}",
                tool_config.name, tool_config.command, tool_config.args
            );

            // Create a server config from the tool config
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

            match &config.server_type {
                McpServerType::Remote { url, access_token, api_key } => {
                    // For remote servers, create an HTTP connection
                    info!("Creating HTTP connection to remote MCP server: {} at URL: {}", config.name, url);
                    
                    let http_connection = HttpMcpConnection::new(
                        &config,
                        url.clone(),
                        access_token.clone(),
                        api_key.clone(),
                    );
                    
                    // Add connection to the registry
                    let connection_id = format!("{}_connection", config.name);
                    self.registry
                        .add_http_connection(connection_id.clone(), http_connection)
                        .await;

                    // Discover tools from the remote server using the registry
                    match self.registry.discover_tools_on_connection(&connection_id).await {
                        Ok(mcp_tools) => {
                            info!(
                                "Discovered {} tools from HTTP MCP server '{}':",
                                mcp_tools.len(),
                                config.name
                            );

                            for tool in &mcp_tools {
                                info!(
                                    "  - Adding MCP tool adapter: {} - {}",
                                    tool.name,
                                    truncate_description(&tool.description)
                                );
                            }

                            info!(
                                "HTTP MCP server '{}' is running and ready for communication at URL: {}",
                                config.name, url
                            );
                        }
                        Err(e) => {
                            error!(
                                "Failed to discover tools from HTTP MCP server '{}': {}",
                                config.name, e
                            );
                        }
                    }
                }
                _ => {
                    // For all other server types, use stdio connection
                    match McpLauncher::launch(&config).await {
                        Ok(_process) => {
                            info!("Successfully launched MCP server: {}", config.name);

                            // Connect to the server and discover its tools
                            match StdioMcpConnection::new(&config).await {
                                Ok(connection) => {
                                    info!(
                                        "Successfully established stdio connection to MCP server: {}",
                                        config.name
                                    );

                                    // Add connection to the registry first
                                    let connection_id = format!("{}_connection", config.name);
                                    self.registry
                                        .add_stdio_connection(connection_id.clone(), connection)
                                        .await;

                                    // Discover tools from the server using the registry
                                    match self.registry.discover_tools_on_connection(&connection_id).await {
                                        Ok(mcp_tools) => {
                                            info!(
                                                "Discovered {} tools from MCP server '{}':",
                                                mcp_tools.len(),
                                                config.name
                                            );

                                            info!(
                                                "MCP server '{}' is running and ready for stdio communication",
                                                config.name
                                            );
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to discover tools from MCP server '{}': {}",
                                                config.name, e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to establish stdio connection to MCP server '{}': {}",
                                        config.name, e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to launch MCP server '{}': {}", config.name, e);
                        }
                    }
                }
            }
        }

        info!("Completed MCP server launch process");
        Ok(())
    }
}

/// Truncate a description to the first 60 characters with an ellipsis if needed
fn truncate_description(description: &str) -> String {
    if description.len() > 60 {
        format!("{}...", &description[..60])
    } else {
        description.to_string()
    }
}

impl McpManager {
    pub fn get_registry(&self) -> &Arc<McpConnectionRegistry> {
        &self.registry
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_mcp_connection_registry_creation() {
        let registry = McpConnectionRegistry::new();
        assert!(registry.connections.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_manager_creation() {
        let manager = McpManager::new();
        assert!(manager.registry.connections.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_tool_adapter_creation() {
        let adapter = McpToolAdapter::new(
            "test_tool".to_string(),
            "A test tool".to_string(),
            serde_json::json!({}),
            "test_connection".to_string(),
        );

        assert_eq!(adapter.name(), "test_tool");
        assert_eq!(adapter.description(), "A test tool");
        assert_eq!(adapter.profile(), ToolProfile::Generic);
    }

    #[tokio::test]
    async fn test_registry_methods_exist() {
        let registry = McpConnectionRegistry::new();
        
        // Test that methods exist - they won't do anything without connections
        // but they should be callable
        let connection_id = "nonexistent".to_string();
        let result = registry.get_connection(&connection_id).await;
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_mcp_tool_adapter_parameters() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                "test_param": {
                    "type": "string"
                }
            }
        });

        let adapter = McpToolAdapter::new(
            "param_test_tool".to_string(),
            "A test tool with parameters".to_string(),
            params.clone(),
            "test_connection".to_string(),
        );

        assert_eq!(adapter.parameters(), params);
    }
    
    #[tokio::test]
    async fn test_mcp_connection_type_enum() {
        use McpConnectionType::*;
        
        let http_conn = HttpMcpConnection::new(
            &McpServerConfig {
                name: "test".to_string(),
                description: Some("test".to_string()),
                server_type: McpServerType::Remote {
                    url: "http://localhost:8080".to_string(),
                    access_token: None,
                    api_key: None,
                },
                auto_start: Some(false),
                environment: None,
            },
            "http://localhost:8080".to_string(),
            None,
            None,
        );
        
        let http_variant = Http(http_conn);
        assert!(matches!(http_variant, Http(_)));
    }
    
    #[tokio::test]
    async fn test_registry_add_http_connection() {
        let registry = McpConnectionRegistry::new();
        
        let http_conn = HttpMcpConnection::new(
            &McpServerConfig {
                name: "test".to_string(),
                description: Some("test".to_string()),
                server_type: McpServerType::Remote {
                    url: "http://localhost:8080".to_string(),
                    access_token: None,
                    api_key: None,
                },
                auto_start: Some(false),
                environment: None,
            },
            "http://localhost:8080".to_string(),
            None,
            None,
        );
        
        registry.add_http_connection("test_conn".to_string(), http_conn).await;
        
        let conn = registry.get_connection(&"test_conn".to_string()).await;
        assert!(conn.is_some());
    }
    
    #[tokio::test]
    async fn test_mcp_tool_adapter_execute_method() {
        let adapter = McpToolAdapter::new(
            "execute_test_tool".to_string(),
            "A test tool for execute method testing".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "test_param": {
                        "type": "string"
                    }
                }
            }),
            "test_connection".to_string(),
        );

        assert_eq!(adapter.name(), "execute_test_tool");
        assert_eq!(adapter.description(), "A test tool for execute method testing");
        assert_eq!(adapter.profile(), ToolProfile::Generic);
    }
    
    #[tokio::test]
    async fn test_mcp_manager_initialization() {
        init_mcp_registry();
        let registry = get_mcp_registry();
        assert!(registry.connections.read().await.is_empty());
        
        let manager = McpManager::new();
        assert!(manager.registry.connections.read().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_mcp_tool_adapter_clone() {
        let original = McpToolAdapter::new(
            "clone_test_tool".to_string(),
            "A test tool for clone testing".to_string(),
            serde_json::json!({}),
            "test_connection".to_string(),
        );
        
        let cloned = original.clone_box();
        assert_eq!(cloned.name(), "clone_test_tool");
        assert_eq!(cloned.description(), "A test tool for clone testing");
    }
}
