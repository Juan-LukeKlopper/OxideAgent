use OxideAgent::core::mcp::config::{McpServerConfig, McpServerType};
use OxideAgent::core::mcp::connection::McpToolDefinition;
use OxideAgent::core::mcp::http::HttpMcpConnection;
use serde::{Deserialize, Serialize};
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

// Private helpers for tests
#[derive(Serialize, Deserialize, Debug)]
struct ToolsListResult {
    tools: Vec<McpToolDefinition>,
}

fn truncate_description(description: &str) -> String {
    if description.len() > 60 {
        format!("{}...", &description[..60])
    } else {
        description.to_string()
    }
}
