//! Integration tests for the MCP configuration functionality.

use OxideAgent::core::mcp::config::{McpConfigFile, McpServerConfig, McpServerType};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_file_roundtrip() {
    let temp_dir = TempDir::new().unwrap();

    // Create a configuration with all server types
    let mut config = McpConfigFile::new();

    // Add a remote server
    config.add_server(McpServerConfig {
        name: "remote-server".to_string(),
        description: Some("A remote MCP server".to_string()),
        server_type: McpServerType::Remote {
            url: "https://example.com/mcp".to_string(),
            access_token: Some("test-token".to_string()),
            api_key: None,
        },
        auto_start: Some(true),
        environment: None,
    });

    // Test saving and loading JSON
    let json_path = temp_dir.path().join("test_config.json");
    config.save_to_file(&json_path).unwrap();
    assert!(json_path.exists());

    let loaded_config = McpConfigFile::load_from_file(&json_path).unwrap();
    assert_eq!(loaded_config.version, config.version);
    assert_eq!(loaded_config.servers.len(), config.servers.len());

    // Test saving and loading TOML
    let toml_path = temp_dir.path().join("test_config.toml");
    config.save_to_file(&toml_path).unwrap();
    assert!(toml_path.exists());

    let loaded_config = McpConfigFile::load_from_file(&toml_path).unwrap();
    assert_eq!(loaded_config.version, config.version);
    assert_eq!(loaded_config.servers.len(), config.servers.len());

    // Test saving and loading YAML
    let yaml_path = temp_dir.path().join("test_config.yaml");
    config.save_to_file(&yaml_path).unwrap();
    assert!(yaml_path.exists());

    let loaded_config = McpConfigFile::load_from_file(&yaml_path).unwrap();
    assert_eq!(loaded_config.version, config.version);
    assert_eq!(loaded_config.servers.len(), config.servers.len());
}

#[test]
fn test_config_file_examples() {
    // Test with example JSON configuration
    let json_content = r#"{
        "version": "1.0",
        "servers": [
            {
                "name": "strava",
                "description": "Strava MCP server for fitness data",
                "type": "npm",
                "package": "@modelcontextprotocol/server-strava",
                "command": "start",
                "autoStart": true
            },
            {
                "name": "github",
                "description": "GitHub MCP server",
                "type": "remote",
                "url": "https://api.github.com/mcp",
                "accessToken": "your_github_token"
            }
        ]
    }"#;

    let temp_dir = TempDir::new().unwrap();
    let json_path = temp_dir.path().join("example.json");
    fs::write(&json_path, json_content).unwrap();

    let config = McpConfigFile::load_from_file(&json_path).unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.servers.len(), 2);

    // Check Strava server
    let strava_server = config.get_server("strava").unwrap();
    assert_eq!(
        strava_server.description.as_ref().unwrap(),
        "Strava MCP server for fitness data"
    );
    assert_eq!(strava_server.auto_start, Some(true));

    match &strava_server.server_type {
        McpServerType::Npm {
            package, command, ..
        } => {
            assert_eq!(package, "@modelcontextprotocol/server-strava");
            assert_eq!(command.as_ref().unwrap(), "start");
        }
        _ => panic!("Expected npm server type for Strava"),
    }

    // Check GitHub server
    let github_server = config.get_server("github").unwrap();
    assert_eq!(
        github_server.description.as_ref().unwrap(),
        "GitHub MCP server"
    );

    match &github_server.server_type {
        McpServerType::Remote {
            url, access_token, ..
        } => {
            assert_eq!(url, "https://api.github.com/mcp");
            assert_eq!(access_token.as_ref().unwrap(), "your_github_token");
        }
        _ => panic!("Expected remote server type for GitHub"),
    }
}
