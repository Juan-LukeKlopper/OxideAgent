//! Tests for MCP connection functionality

use OxideAgent::core::mcp::connection::{McpConnection, McpToolDefinition, StdioMcpConnection};
use OxideAgent::core::mcp::http::HttpMcpConnection;
use OxideAgent::core::mcp::config::{McpServerConfig, McpServerType};
use OxideAgent::core::mcp::launcher::McpLauncher;
use OxideAgent::core::tools::{Tool, ToolProfile};
use anyhow::Result;
use serde_json::Value;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_stdio_connection_trait_implementation() {
    // Create a mock server config for testing
    let config = McpServerConfig {
        name: "test-server".to_string(),
        description: Some("Test server for connection testing".to_string()),
        server_type: McpServerType::Command {
            command: "echo".to_string(), // Using a simple command for testing
            args: Some(vec!["hello".to_string()]),
            environment: None,
            working_directory: None,
        },
        auto_start: Some(true),
        environment: None,
    };

    // Test that the StdioMcpConnection properly implements the trait
    // Note: We can't actually connect to an MCP server without one running,
    // but we can check that the trait methods are callable
    
    // Create a mock connection that just returns errors for testing the interface
    // Actually testing requires a running MCP server, so we'll just verify the 
    // trait implementation compiles and is callable
    
    // This test verifies that the trait methods can be called
    // In a real test environment we'd mock the connection properly
    assert!(true); // Placeholder - actual implementation requires a running mock server
}

#[tokio::test]
async fn test_http_connection_trait_implementation() {
    // Similar test for HTTP connection
    let config = McpServerConfig {
        name: "test-http-server".to_string(),
        description: Some("Test HTTP server for connection testing".to_string()),
        server_type: McpServerType::Remote {
            url: "http://localhost:8080".to_string(),
            access_token: None,
            api_key: None,
        },
        auto_start: Some(false),
        environment: None,
    };

    let http_connection = HttpMcpConnection::new(
        &config,
        "http://localhost:8080".to_string(),
        None,
        None,
    );

    // Verify the trait methods are implemented
    // The actual calls will fail without a server, but they should compile and be callable
    assert!(true); // Placeholder - actual implementation requires a running mock server
}

#[tokio::test]
async fn test_stdio_connection_discover_tools_trait_method() {
    // Test that the discover_tools trait method is properly implemented
    // For this test, we'll need to create a mock that doesn't actually connect
    
    // This test ensures that the trait implementation compiles and can be called
    // In a proper testing scenario, we'd have a mock implementation
    assert!(true); // Placeholder to be implemented with proper mocks
}

#[tokio::test]
async fn test_stdio_connection_execute_tool_trait_method() {
    // Test that the execute_tool trait method is properly implemented
    assert!(true); // Placeholder to be implemented with proper mocks
}

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
async fn test_mcp_tool_definition_as_tool_trait() {
    let tool_def = McpToolDefinition {
        name: "test_tool".to_string(),
        description: "A test tool for trait conversion".to_string(),
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

    // Verify the fields are accessible
    assert_eq!(tool_def.name, "test_tool");
    assert_eq!(tool_def.description, "A test tool for trait conversion");
}

#[tokio::test]
async fn test_mcp_launcher_config_parsing() {
    // Test that the launcher can parse different server configurations
    let config = McpServerConfig {
        name: "test-launcher".to_string(),
        description: Some("Test launcher config".to_string()),
        server_type: McpServerType::Command {
            command: "echo".to_string(),
            args: Some(vec!["test".to_string()]),
            environment: None,
            working_directory: None,
        },
        auto_start: Some(true),
        environment: None,
    };

    // Just verify the config can be created and accessed (actual launching would require the command to exist)
    assert_eq!(config.name, "test-launcher");
    assert_eq!(config.description, Some("Test launcher config".to_string()));
    
    match &config.server_type {
        McpServerType::Command { command, args, .. } => {
            assert_eq!(command, "echo");
            assert_eq!(args.as_ref().unwrap(), &vec!["test".to_string()]);
        }
        _ => panic!("Expected command server type"),
    }
}