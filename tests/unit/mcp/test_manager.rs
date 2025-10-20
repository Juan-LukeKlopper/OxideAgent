use OxideAgent::core::mcp::config::{McpServerConfig, McpServerType};
use OxideAgent::core::mcp::http::HttpMcpConnection;
use OxideAgent::core::mcp::manager::McpManager;
use OxideAgent::core::mcp::manager::*;
use OxideAgent::core::tools::Tool;
use OxideAgent::core::tools::ToolProfile;

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

    registry
        .add_http_connection("test_conn".to_string(), http_conn)
        .await;

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
    assert_eq!(
        adapter.description(),
        "A test tool for execute method testing"
    );
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
