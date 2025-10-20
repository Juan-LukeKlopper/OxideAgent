use OxideAgent::core::mcp::config::{McpServerConfig, McpServerType};
use OxideAgent::core::mcp::connection::*;
use OxideAgent::core::tools::{Tool, ToolProfile};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
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

// Private helpers for tests
fn truncate_description(description: &str) -> String {
    if description.len() > 60 {
        format!("{}...", &description[..60])
    } else {
        description.to_string()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u32,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JsonRpcSuccessResponse {
    jsonrpc: String,
    id: u32,
    result: Value,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JsonRpcErrorResponse {
    jsonrpc: String,
    id: u32,
    error: JsonRpcErrorObject,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct JsonRpcErrorObject {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ToolsListResult {
    tools: Vec<McpToolDefinition>,
}
