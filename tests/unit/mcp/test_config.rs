//! Tests for the MCP configuration management module.

use OxideAgent::core::mcp::config::{McpConfigFile, McpServerConfig, McpServerType};
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_json_config_loading() {
    let json_content = r#"{
        "version": "1.0",
        "servers": [
            {
                "name": "test-server",
                "type": "remote",
                "url": "https://example.com/mcp",
                "accessToken": "test-token"
            }
        ]
    }"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(json_content.as_bytes()).unwrap();

    let config = McpConfigFile::load_from_file(temp_file.path()).unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.servers.len(), 1);

    let server = &config.servers[0];
    assert_eq!(server.name, "test-server");
    match &server.server_type {
        McpServerType::Remote {
            url,
            access_token,
            api_key,
        } => {
            assert_eq!(url, "https://example.com/mcp");
            assert_eq!(access_token.as_ref().unwrap(), "test-token");
            assert!(api_key.is_none());
        }
        _ => panic!("Expected remote server type"),
    }
}

#[test]
fn test_toml_config_loading() {
    let toml_content = r#"
        version = "1.0"
        
        [[servers]]
        name = "test-server"
        type = "npm"
        package = "test-package"
    "#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    let temp_path = temp_file.into_temp_path();
    temp_path.persist("/tmp/test_mcp.toml").unwrap();

    let config = McpConfigFile::load_from_file("/tmp/test_mcp.toml").unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.servers.len(), 1);

    let server = &config.servers[0];
    assert_eq!(server.name, "test-server");
    match &server.server_type {
        McpServerType::Npm { package, .. } => {
            assert_eq!(package, "test-package");
        }
        _ => panic!("Expected npm server type"),
    }

    // Clean up
    let _ = std::fs::remove_file("/tmp/test_mcp.toml");
}

#[test]
fn test_yaml_config_loading() {
    let yaml_content = r#"
        version: "1.0"
        servers:
          - name: "test-server"
            type: "docker"
            image: "test-image"
    "#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    let temp_path = temp_file.into_temp_path();
    temp_path.persist("/tmp/test_mcp.yaml").unwrap();

    let config = McpConfigFile::load_from_file("/tmp/test_mcp.yaml").unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.servers.len(), 1);

    let server = &config.servers[0];
    assert_eq!(server.name, "test-server");
    match &server.server_type {
        McpServerType::Docker { image, .. } => {
            assert_eq!(image, "test-image");
        }
        _ => panic!("Expected docker server type"),
    }

    // Clean up
    let _ = std::fs::remove_file("/tmp/test_mcp.yaml");
}

#[test]
fn test_config_management() {
    let mut config = McpConfigFile::new();

    // Add a server
    let server = McpServerConfig {
        name: "test-server".to_string(),
        description: Some("A test server".to_string()),
        server_type: McpServerType::Remote {
            url: "https://example.com/mcp".to_string(),
            access_token: Some("test-token".to_string()),
            api_key: None,
        },
        auto_start: Some(true),
        environment: None,
    };

    config.add_server(server);
    assert_eq!(config.servers.len(), 1);
    assert_eq!(config.list_server_names(), vec!["test-server"]);

    // Get server by name
    let retrieved = config.get_server("test-server").unwrap();
    assert_eq!(retrieved.name, "test-server");

    // Remove server
    assert!(config.remove_server("test-server"));
    assert_eq!(config.servers.len(), 0);
    assert!(!config.remove_server("non-existent"));
}

#[test]
fn test_docker_server_config() {
    let mut config = McpConfigFile::new();

    let mut env_vars = HashMap::new();
    env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

    let server = McpServerConfig {
        name: "docker-test".to_string(),
        description: Some("A docker test server".to_string()),
        server_type: McpServerType::Docker {
            image: "test-image:latest".to_string(),
            command: Some(vec!["start".to_string()]),
            ports: Some(vec!["3000:3000".to_string()]),
            volumes: Some(vec!["/host:/container".to_string()]),
            environment: Some(env_vars),
        },
        auto_start: Some(false),
        environment: None,
    };

    config.add_server(server);
    assert_eq!(config.servers.len(), 1);

    let retrieved = config.get_server("docker-test").unwrap();
    match &retrieved.server_type {
        McpServerType::Docker {
            image,
            command,
            ports,
            volumes,
            environment,
        } => {
            assert_eq!(image, "test-image:latest");
            assert_eq!(command.as_ref().unwrap(), &vec!["start".to_string()]);
            assert_eq!(ports.as_ref().unwrap(), &vec!["3000:3000".to_string()]);
            assert_eq!(
                volumes.as_ref().unwrap(),
                &vec!["/host:/container".to_string()]
            );
            assert_eq!(
                environment.as_ref().unwrap().get("TEST_VAR").unwrap(),
                "test_value"
            );
        }
        _ => panic!("Expected docker server type"),
    }
}

#[test]
fn test_npm_server_config() {
    let mut config = McpConfigFile::new();

    let server = McpServerConfig {
        name: "npm-test".to_string(),
        description: None,
        server_type: McpServerType::Npm {
            package: "@test/package".to_string(),
            command: Some("serve".to_string()),
            args: Some(vec!["--port".to_string(), "3000".to_string()]),
            environment: None,
        },
        auto_start: Some(true),
        environment: None,
    };

    config.add_server(server);
    assert_eq!(config.servers.len(), 1);

    let retrieved = config.get_server("npm-test").unwrap();
    match &retrieved.server_type {
        McpServerType::Npm {
            package,
            command,
            args,
            environment,
        } => {
            assert_eq!(package, "@test/package");
            assert_eq!(command.as_ref().unwrap(), "serve");
            assert_eq!(
                args.as_ref().unwrap(),
                &vec!["--port".to_string(), "3000".to_string()]
            );
            assert!(environment.is_none());
        }
        _ => panic!("Expected npm server type"),
    }
}

#[test]
fn test_command_server_config() {
    let mut config = McpConfigFile::new();

    let mut env_vars = HashMap::new();
    env_vars.insert("PATH".to_string(), "/usr/local/bin".to_string());

    let server = McpServerConfig {
        name: "command-test".to_string(),
        description: None,
        server_type: McpServerType::Command {
            command: "python".to_string(),
            args: Some(vec!["server.py".to_string()]),
            environment: Some(env_vars),
            working_directory: Some("/app".to_string()),
        },
        auto_start: Some(false),
        environment: None,
    };

    config.add_server(server);
    assert_eq!(config.servers.len(), 1);

    let retrieved = config.get_server("command-test").unwrap();
    match &retrieved.server_type {
        McpServerType::Command {
            command,
            args,
            environment,
            working_directory,
        } => {
            assert_eq!(command, "python");
            assert_eq!(args.as_ref().unwrap(), &vec!["server.py".to_string()]);
            assert_eq!(
                environment.as_ref().unwrap().get("PATH").unwrap(),
                "/usr/local/bin"
            );
            assert_eq!(working_directory.as_ref().unwrap(), "/app");
        }
        _ => panic!("Expected command server type"),
    }
}
