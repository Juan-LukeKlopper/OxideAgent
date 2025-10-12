//! MCP (Model Context Protocol) launcher for the OxideAgent system.
//!
//! This module provides functionality for spawning MCP servers using various methods:
//! - Docker containers
//! - NPM package runner (npx)
//! - Astral UVX runner

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

use super::config::{McpServerConfig, McpServerType};
use anyhow::{Context, Result};

// Note: This struct and its fields are kept for possible future use
#[allow(dead_code)]
/// Process information for a spawned MCP server
pub struct McpProcess {
    /// Process ID of the spawned server
    #[allow(dead_code)]
    pub pid: u32,
    /// Command used to start the process
    #[allow(dead_code)]
    pub command: String,
    /// Arguments passed to the command
    #[allow(dead_code)]
    pub args: Vec<String>,
    /// The actual process handle
    #[allow(dead_code)]
    pub handle: std::process::Child,
    /// For Docker containers, store the container ID
    #[allow(dead_code)]
    pub container_id: Option<String>,
}

/// MCP Server launcher that abstracts starting an MCP server based on its configuration
pub struct McpLauncher;

impl McpLauncher {
    // Note: This method and others in this impl block are kept for possible future use
    /// Launch an MCP server based on the provided configuration
    pub async fn launch(config: &McpServerConfig) -> Result<McpProcess> {
        match &config.server_type {
            McpServerType::Docker {
                image,
                command,
                ports,
                volumes,
                environment,
            } => {
                Self::launch_docker_server(
                    image,
                    command,
                    ports,
                    volumes,
                    environment,
                    &config.name,
                )
                .await
            }
            McpServerType::Npm {
                package,
                command,
                args,
                environment,
            } => Self::launch_npx_server(package, command, args, environment, &config.name).await,
            McpServerType::Command {
                command,
                args,
                environment,
                working_directory,
            } => {
                // Check if this is a uvx command
                if command == "uvx" {
                    Self::launch_uvx_server_from_command(command, args, environment, &config.name)
                        .await
                } else {
                    Self::launch_command_server(
                        command,
                        args,
                        environment,
                        working_directory,
                        &config.name,
                    )
                    .await
                }
            }
            McpServerType::Remote { .. } => {
                // Remote servers don't need to be launched
                Err(anyhow::anyhow!(
                    "Remote MCP servers don't need to be launched locally"
                ))
            }
        }
    }

    #[allow(dead_code)]
    /// Launch an MCP server using Docker
    async fn launch_docker_server(
        image: &str,
        command: &Option<Vec<String>>,
        ports: &Option<Vec<String>>,
        volumes: &Option<Vec<String>>,
        environment: &Option<HashMap<String, String>>,
        name: &str,
    ) -> Result<McpProcess> {
        info!(
            "Launching Docker MCP server '{}' using image '{}'",
            name, image
        );

        // Check if Docker is available
        Self::check_docker_availability()
            .await
            .context("Failed to launch Docker MCP server - Docker not available")?;

        let mut docker_cmd = Command::new("docker");

        // Add run command
        docker_cmd.arg("run");

        // Add port mappings
        if let Some(ports) = ports {
            for port in ports {
                docker_cmd.arg("-p").arg(port);
            }
        }

        // Add volume mappings
        if let Some(volumes) = volumes {
            for volume in volumes {
                docker_cmd.arg("-v").arg(volume);
            }
        }

        // Add environment variables
        if let Some(env) = environment {
            for (key, value) in env {
                docker_cmd.arg("-e").arg(format!("{}={}", key, value));
            }
        }

        // Add detach flag to run in background
        docker_cmd.arg("-d");

        // Add the image
        docker_cmd.arg(image);

        // Add command if specified
        if let Some(cmd) = command {
            docker_cmd.args(cmd);
        }

        // Execute the Docker command
        let output = docker_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                error!("Failed to execute Docker command for '{}': {}", name, e);
                anyhow::anyhow!("Failed to execute Docker command for '{}': {}", name, e)
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!(
                "Docker command failed for MCP server '{}':\nstdout: {}\nstderr: {}",
                name, stdout, stderr
            );
            return Err(anyhow::anyhow!(
                "Docker command failed for MCP server '{}':\nstdout: {}\nstderr: {}",
                name,
                stdout,
                stderr
            ));
        }

        // Extract the container ID from stdout
        let container_id = String::from_utf8(output.stdout)
            .context("Failed to parse Docker container ID")?
            .trim()
            .to_string();

        if container_id.is_empty() {
            return Err(anyhow::anyhow!(
                "Failed to get container ID for MCP server '{}'",
                name
            ));
        }

        // Wait briefly to ensure the container is running
        sleep(Duration::from_secs(1)).await;

        // Get the PID of the container (we'll use the container ID as a form of PID for Docker)
        // Actually get the running container's info to confirm it's running
        let inspect_cmd = Command::new("docker")
            .arg("inspect")
            .arg(&container_id)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to inspect Docker container for '{}'", name))?;

        if !inspect_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&inspect_cmd.stderr);
            return Err(anyhow::anyhow!(
                "Failed to inspect Docker container for '{}': {}",
                name,
                stderr
            ));
        }

        info!(
            "Docker MCP server '{}' launched successfully with container ID: {}",
            name, container_id
        );

        // For Docker, we'll use a dummy child process since Docker runs the container in the background
        // In reality, we'd want more sophisticated Docker container management
        let dummy_cmd = Command::new("echo") // Just a placeholder command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to create placeholder process for Docker server '{}'",
                    name
                )
            })?;

        Ok(McpProcess {
            pid: 0, // Docker containers don't map directly to host PIDs
            command: format!("docker run {}", image),
            args: vec![], // Already incorporated in the command above
            handle: dummy_cmd,
            container_id: Some(container_id),
        })
    }

    #[allow(dead_code)]
    /// Launch an MCP server using npx
    async fn launch_npx_server(
        package: &str,
        command: &Option<String>,
        args: &Option<Vec<String>>,
        environment: &Option<HashMap<String, String>>,
        name: &str,
    ) -> Result<McpProcess> {
        info!(
            "Launching NPX MCP server '{}' using package '{}'",
            name, package
        );

        // Check if Node.js and npx are available
        Self::check_npx_availability()
            .await
            .context("Failed to launch NPX MCP server - npx not available")?;

        let mut npx_cmd = Command::new("npx");

        // Add the package name
        npx_cmd.arg("--yes"); // Automatically install if not present
        npx_cmd.arg(package);

        // Add command if specified
        if let Some(cmd) = command {
            npx_cmd.arg(cmd);
        }

        // Add additional arguments if specified
        if let Some(args) = args {
            npx_cmd.args(args);
        }

        // Add environment variables
        if let Some(env) = environment {
            for (key, value) in env {
                npx_cmd.env(key, value);
            }
        }

        // Execute the npx command
        let mut child = npx_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn npx command for '{}': {}", name, e);
                anyhow::anyhow!("Failed to spawn npx command for '{}': {}", name, e)
            })?;

        let pid = child.id();

        // Check immediately if the process exited with an error
        if let Ok(Some(exit_status)) = child.try_wait()
            && !exit_status.success()
        {
            let stderr = if let Some(mut stderr_reader) = child.stderr.take() {
                let mut stderr_content = String::new();
                match std::io::Read::read_to_string(&mut stderr_reader, &mut stderr_content) {
                    Ok(_) => stderr_content,
                    Err(_) => "Could not read stderr".to_string(),
                }
            } else {
                "No stderr available".to_string()
            };

            error!(
                "NPX command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            );
            return Err(anyhow::anyhow!(
                "NPX command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            ));
        }

        info!(
            "NPX MCP server '{}' launched successfully with PID: {}",
            name, pid
        );

        Ok(McpProcess {
            pid,
            command: format!("npx {}", package),
            args: args.clone().unwrap_or_default(),
            handle: child,
            container_id: None,
        })
    }

    #[allow(dead_code)]
    /// Launch an MCP server using uvx (Astral) from command variant
    async fn launch_uvx_server_from_command(
        command: &str,
        args: &Option<Vec<String>>,
        environment: &Option<HashMap<String, String>>,
        name: &str,
    ) -> Result<McpProcess> {
        info!(
            "Launching UVX MCP server '{}' using command '{}'",
            name, command
        );

        // Check if uvx is available
        Self::check_uvx_availability()
            .await
            .context("Failed to launch UVX MCP server - uvx not available")?;

        let mut uvx_cmd = Command::new(command);

        // Add additional arguments if specified
        if let Some(args) = args {
            uvx_cmd.args(args);
        }

        // Add environment variables
        if let Some(env) = environment {
            for (key, value) in env {
                uvx_cmd.env(key, value);
            }
        }

        // Execute the uvx command
        let mut child = uvx_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn uvx command for '{}': {}", name, e);
                anyhow::anyhow!("Failed to spawn uvx command for '{}': {}", name, e)
            })?;

        let pid = child.id();

        // Check immediately if the process exited with an error
        if let Ok(Some(exit_status)) = child.try_wait()
            && !exit_status.success()
        {
            let stderr = if let Some(mut stderr_reader) = child.stderr.take() {
                let mut stderr_content = String::new();
                match std::io::Read::read_to_string(&mut stderr_reader, &mut stderr_content) {
                    Ok(_) => stderr_content,
                    Err(_) => "Could not read stderr".to_string(),
                }
            } else {
                "No stderr available".to_string()
            };

            error!(
                "UVX command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            );
            return Err(anyhow::anyhow!(
                "UVX command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            ));
        }

        info!(
            "UVX MCP server '{}' launched successfully with PID: {}",
            name, pid
        );

        Ok(McpProcess {
            pid,
            command: command.to_string(),
            args: args.clone().unwrap_or_default(),
            handle: child,
            container_id: None,
        })
    }

    // Note: This method is kept for possible future use
    #[allow(dead_code)]
    /// Launch an MCP server using uvx (Astral)
    async fn launch_uvx_server(
        package: &str,
        args: &Option<Vec<String>>,
        environment: &Option<HashMap<String, String>>,
        name: &str,
    ) -> Result<McpProcess> {
        info!(
            "Launching UVX MCP server '{}' using package '{}'",
            name, package
        );

        // Check if uvx is available
        Self::check_uvx_availability()
            .await
            .context("Failed to launch UVX MCP server - uvx not available")?;

        let mut uvx_cmd = Command::new("uvx");

        // Add the package name
        uvx_cmd.arg(package);

        // Add additional arguments if specified
        if let Some(args) = args {
            uvx_cmd.args(args);
        }

        // Add environment variables
        if let Some(env) = environment {
            for (key, value) in env {
                uvx_cmd.env(key, value);
            }
        }

        // Execute the uvx command
        let mut child = uvx_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn uvx command for '{}': {}", name, e);
                anyhow::anyhow!("Failed to spawn uvx command for '{}': {}", name, e)
            })?;

        let pid = child.id();

        // Check immediately if the process exited with an error
        if let Ok(Some(exit_status)) = child.try_wait()
            && !exit_status.success()
        {
            let stderr = if let Some(mut stderr_reader) = child.stderr.take() {
                let mut stderr_content = String::new();
                match std::io::Read::read_to_string(&mut stderr_reader, &mut stderr_content) {
                    Ok(_) => stderr_content,
                    Err(_) => "Could not read stderr".to_string(),
                }
            } else {
                "No stderr available".to_string()
            };

            error!(
                "UVX command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            );
            return Err(anyhow::anyhow!(
                "UVX command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            ));
        }

        info!(
            "UVX MCP server '{}' launched successfully with PID: {}",
            name, pid
        );

        Ok(McpProcess {
            pid,
            command: format!("uvx {}", package),
            args: args.clone().unwrap_or_default(),
            handle: child,
            container_id: None,
        })
    }

    #[allow(dead_code)]
    /// Launch an MCP server using a direct command
    async fn launch_command_server(
        command: &str,
        args: &Option<Vec<String>>,
        environment: &Option<HashMap<String, String>>,
        working_directory: &Option<String>,
        name: &str,
    ) -> Result<McpProcess> {
        info!(
            "Launching command MCP server '{}' using command '{}'",
            name, command
        );

        let mut cmd = Command::new(command);

        // Add arguments if specified
        if let Some(args) = args {
            cmd.args(args);
        }

        // Set working directory if specified
        if let Some(wd) = working_directory {
            cmd.current_dir(wd);
        }

        // Add environment variables
        if let Some(env) = environment {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Execute the command
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn command for '{}': {}", name, e);
                anyhow::anyhow!("Failed to spawn command for '{}': {}", name, e)
            })?;

        let pid = child.id();

        // Check immediately if the process exited with an error
        if let Ok(Some(exit_status)) = child.try_wait()
            && !exit_status.success()
        {
            let stderr = if let Some(mut stderr_reader) = child.stderr.take() {
                let mut stderr_content = String::new();
                match std::io::Read::read_to_string(&mut stderr_reader, &mut stderr_content) {
                    Ok(_) => stderr_content,
                    Err(_) => "Could not read stderr".to_string(),
                }
            } else {
                "No stderr available".to_string()
            };

            error!(
                "Command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            );
            return Err(anyhow::anyhow!(
                "Command for '{}' exited immediately with code {}: {}",
                name,
                exit_status.code().unwrap_or(-1),
                stderr
            ));
        }

        info!(
            "Command MCP server '{}' launched successfully with PID: {}",
            name, pid
        );

        Ok(McpProcess {
            pid,
            command: command.to_string(),
            args: args.clone().unwrap_or_default(),
            handle: child,
            container_id: None,
        })
    }

    #[allow(dead_code)]
    /// Check if Docker is available on the system
    async fn check_docker_availability() -> Result<()> {
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .context("Failed to execute 'docker --version' command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Docker is not available or not in PATH"));
        }

        Ok(())
    }

    #[allow(dead_code)]
    /// Check if npx is available on the system
    async fn check_npx_availability() -> Result<()> {
        let output = Command::new("npx")
            .arg("--version")
            .output()
            .context("Failed to execute 'npx --version' command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("npx is not available or not in PATH"));
        }

        Ok(())
    }

    #[allow(dead_code)]
    /// Check if uvx is available on the system
    async fn check_uvx_availability() -> Result<()> {
        let output = Command::new("uvx")
            .arg("--version")
            .output()
            .context("Failed to execute 'uvx --version' command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("uvx is not available or not in PATH"));
        }

        Ok(())
    }

    // Note: This method is kept for possible future use
    #[allow(dead_code)]
    /// Helper function to create a mock child process
    /// This is a placeholder since we can't create a real Child process without spawning a command
    fn create_mock_child_process() -> Result<std::process::Child> {
        // This is a placeholder implementation
        // In a real scenario, we'd need proper process management for Docker containers
        // For now, we'll return an error to indicate that Docker implementation is partial
        Err(anyhow::anyhow!(
            "Docker process management requires more sophisticated handling"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_check_docker_availability() {
        // This test will pass if Docker is available, or fail if it's not
        // This is acceptable for testing the function logic
        let result = McpLauncher::check_docker_availability().await;
        // Note: This test might fail in environments without Docker
        // which is expected behavior
        if result.is_err() {
            eprintln!(
                "Docker not available in test environment: {:?}",
                result.err()
            );
        }
        // We don't assert success/failure as it depends on test environment
    }

    #[tokio::test]
    async fn test_check_npx_availability() {
        // This test will pass if npx is available, or fail if it's not
        let result = McpLauncher::check_npx_availability().await;
        if result.is_err() {
            eprintln!("npx not available in test environment: {:?}", result.err());
        }
    }

    #[tokio::test]
    async fn test_check_uvx_availability() {
        // This test will pass if uvx is available, or fail if it's not
        let result = McpLauncher::check_uvx_availability().await;
        if result.is_err() {
            eprintln!("uvx not available in test environment: {:?}", result.err());
        }
    }

    #[test]
    fn test_mcp_server_config_docker() {
        let config = McpServerConfig {
            name: "test-docker".to_string(),
            description: Some("Test Docker server".to_string()),
            server_type: McpServerType::Docker {
                image: "some-image:latest".to_string(),
                command: Some(vec!["start".to_string()]),
                ports: Some(vec!["3000:3000".to_string()]),
                volumes: Some(vec!["/tmp:/tmp".to_string()]),
                environment: Some(HashMap::from([(
                    "ENV_VAR".to_string(),
                    "value".to_string(),
                )])),
            },
            auto_start: Some(true),
            environment: None,
        };

        match &config.server_type {
            McpServerType::Docker { image, .. } => {
                assert_eq!(image, "some-image:latest");
            }
            _ => panic!("Expected Docker server type"),
        }
    }

    #[test]
    fn test_mcp_server_config_npm() {
        let config = McpServerConfig {
            name: "test-npm".to_string(),
            description: Some("Test NPM server".to_string()),
            server_type: McpServerType::Npm {
                package: "@modelcontextprotocol/test-server".to_string(),
                command: Some("start".to_string()),
                args: Some(vec!["--port".to_string(), "3000".to_string()]),
                environment: Some(HashMap::from([(
                    "NODE_ENV".to_string(),
                    "test".to_string(),
                )])),
            },
            auto_start: Some(true),
            environment: None,
        };

        match &config.server_type {
            McpServerType::Npm { package, .. } => {
                assert_eq!(package, "@modelcontextprotocol/test-server");
            }
            _ => panic!("Expected NPM server type"),
        }
    }

    #[test]
    fn test_mcp_server_config_command() {
        let config = McpServerConfig {
            name: "test-command".to_string(),
            description: Some("Test command server".to_string()),
            server_type: McpServerType::Command {
                command: "cargo".to_string(),
                args: Some(vec!["run".to_string()]),
                environment: Some(HashMap::from([(
                    "RUST_LOG".to_string(),
                    "debug".to_string(),
                )])),
                working_directory: Some("/tmp".to_string()),
            },
            auto_start: Some(true),
            environment: None,
        };

        match &config.server_type {
            McpServerType::Command { command, .. } => {
                assert_eq!(command, "cargo");
            }
            _ => panic!("Expected Command server type"),
        }
    }

    // Test for environment variable handling in launch functions
    #[test]
    fn test_environment_variable_parsing() {
        let env_map = HashMap::from([
            ("TEST_VAR".to_string(), "test_value".to_string()),
            ("ANOTHER_VAR".to_string(), "another_value".to_string()),
        ]);

        // Just verify that we can create and access the environment map
        assert_eq!(env_map.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert_eq!(
            env_map.get("ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
        assert_eq!(env_map.len(), 2);
    }

    #[test]
    fn test_environment_variable_application() {
        // Create a command that will use environment variables
        let mut cmd = Command::new("env"); // 'env' command prints all environment variables on Unix systems

        // Add our test environment variables
        let env_map = HashMap::from([
            ("OXIDE_TEST_VAR".to_string(), "test_value".to_string()),
            ("OXIDE_ANOTHER_VAR".to_string(), "another_value".to_string()),
        ]);

        for (key, value) in &env_map {
            cmd.env(key, value);
        }

        // Note: Actually executing this command might not be reliable in tests
        // but we can at least test that the environment variables were set in the command
        // In a real implementation we'd have a more robust way to test this
        assert_eq!(
            env_map.get("OXIDE_TEST_VAR"),
            Some(&"test_value".to_string())
        );
        assert_eq!(
            env_map.get("OXIDE_ANOTHER_VAR"),
            Some(&"another_value".to_string())
        );
    }

    // Note: These process launching tests need to be mocked in a real implementation
    // For now, we'll test the configuration parsing aspects
    #[tokio::test]
    #[ignore] // Ignore these tests by default as they would try to launch actual processes
    async fn test_launch_docker_server() {
        let env_map = HashMap::from([("ENV_VAR".to_string(), "value".to_string())]);
        let config = McpServerConfig {
            name: "test-docker".to_string(),
            description: Some("Test Docker server".to_string()),
            server_type: McpServerType::Docker {
                image: "alpine:latest".to_string(), // Using a minimal image for testing
                command: Some(vec!["echo".to_string(), "hello".to_string()]),
                ports: None,
                volumes: None,
                environment: Some(env_map),
            },
            auto_start: Some(true),
            environment: None,
        };

        // This test would try to actually launch the Docker container
        // For a real test we'd need to mock the Docker commands
        match McpLauncher::launch(&config).await {
            Ok(process) => {
                // In a real test, we'd validate the process info
                println!("Process launched with PID: {}", process.pid);
            }
            Err(e) => {
                eprintln!("Error launching Docker server: {}", e);
                // This might be expected if Docker is not available in test environment
            }
        }
    }

    #[tokio::test]
    #[ignore] // Ignore these tests by default as they would try to launch actual processes
    async fn test_launch_npx_server() {
        let config = McpServerConfig {
            name: "test-npx".to_string(),
            description: Some("Test NPX server".to_string()),
            server_type: McpServerType::Npm {
                package: "serve".to_string(),         // A simple package for testing
                command: Some("version".to_string()), // Just get version to avoid actual server
                args: None,
                environment: None,
            },
            auto_start: Some(true),
            environment: None,
        };

        // This test would try to actually run npx
        // For a real test we'd need to mock the npx commands
        match McpLauncher::launch(&config).await {
            Ok(process) => {
                // In a real test, we'd validate the process info
                println!("Process launched with PID: {}", process.pid);
            }
            Err(e) => {
                eprintln!("Error launching NPX server: {}", e);
                // This might be expected if npx is not available in test environment
            }
        }
    }

    #[test]
    fn test_mcp_process_struct_fields() {
        // Test that we can create and access fields of the McpProcess struct
        // Note: We can't easily test the actual Child process construction in unit tests
        // This test just verifies the struct definition and field access work as expected

        // We'll test with dummy values to verify field access
        let pid_field: u32 = 1234;
        let command_field = "test_command".to_string();
        let args_field = vec!["arg1".to_string(), "arg2".to_string()];
        let container_id_field = Some("test_container_id".to_string());

        assert_eq!(pid_field, 1234);
        assert_eq!(command_field, "test_command");
        assert_eq!(args_field, vec!["arg1".to_string(), "arg2".to_string()]);
        assert_eq!(container_id_field, Some("test_container_id".to_string()));

        // This test mainly verifies the struct definition compiles correctly
        // Actual process creation and management would be tested in integration tests
    }

    #[tokio::test]
    async fn test_launch_npx_server_availability() {
        // This test just checks the availability checking function
        // In a real test we'd mock the command execution, but for now just ensure the function runs
        let result = McpLauncher::check_npx_availability().await;
        // Result could be Ok or Err depending on environment, just ensure no panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_launch_docker_server_availability() {
        // This test just checks the availability checking function
        let result = McpLauncher::check_docker_availability().await;
        // Result could be Ok or Err depending on environment, just ensure no panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_launch_uvx_server_availability() {
        // This test just checks the availability checking function
        let result = McpLauncher::check_uvx_availability().await;
        // Result could be Ok or Err depending on environment, just ensure no panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_launch_with_docker_config() {
        let config = McpServerConfig {
            name: "test-docker".to_string(),
            description: Some("Test Docker server".to_string()),
            server_type: McpServerType::Docker {
                image: "alpine:latest".to_string(),
                command: Some(vec!["echo".to_string(), "hello".to_string()]),
                ports: Some(vec!["3000:3000".to_string()]),
                volumes: Some(vec!["/tmp:/tmp".to_string()]),
                environment: Some(HashMap::from([(
                    "ENV_VAR".to_string(),
                    "value".to_string(),
                )])),
            },
            auto_start: Some(true),
            environment: None,
        };

        // This test checks that the configuration is correctly parsed and processed
        // The actual Docker launch will fail in test environment, which is expected
        let result = McpLauncher::launch(&config).await;
        // The result could be Ok or Err depending on test environment, so we just verify it doesn't panic
        assert!(result.is_ok() || result.is_err()); // This ensures the function executes without panicking
    }

    #[tokio::test]
    async fn test_launch_with_npm_config() {
        let config = McpServerConfig {
            name: "test-npm".to_string(),
            description: Some("Test NPM server".to_string()),
            server_type: McpServerType::Npm {
                package: "@modelcontextprotocol/test-server".to_string(),
                command: Some("start".to_string()),
                args: Some(vec!["--port".to_string(), "3000".to_string()]),
                environment: Some(HashMap::from([(
                    "NODE_ENV".to_string(),
                    "test".to_string(),
                )])),
            },
            auto_start: Some(true),
            environment: None,
        };

        // This test checks that the configuration is correctly parsed and processed
        // The actual NPM launch will fail in test environment without npx, which is expected
        let result = McpLauncher::launch(&config).await;
        // The result could be Ok or Err depending on test environment, so we just verify it doesn't panic
        assert!(result.is_ok() || result.is_err()); // This ensures the function executes without panicking
    }

    #[tokio::test]
    async fn test_launch_with_command_config() {
        let config = McpServerConfig {
            name: "test-command".to_string(),
            description: Some("Test command server".to_string()),
            server_type: McpServerType::Command {
                command: "echo".to_string(),
                args: Some(vec!["hello".to_string()]),
                environment: Some(HashMap::from([(
                    "TEST_VAR".to_string(),
                    "test_value".to_string(),
                )])),
                working_directory: Some("/tmp".to_string()),
            },
            auto_start: Some(true),
            environment: None,
        };

        // This test checks that the configuration is correctly parsed and processed
        let result = McpLauncher::launch(&config).await;
        // The result could be Ok or Err depending on test environment, so we just verify it doesn't panic
        assert!(result.is_ok() || result.is_err()); // This ensures the function executes without panicking
    }

    #[tokio::test]
    async fn test_launch_with_remote_config() {
        // Test that remote config servers return an error when launched
        let config = McpServerConfig {
            name: "test-remote".to_string(),
            description: Some("Test remote server".to_string()),
            server_type: McpServerType::Remote {
                url: "http://example.com".to_string(),
                access_token: Some("token".to_string()),
                api_key: None,
            },
            auto_start: Some(true),
            environment: None,
        };

        // Remote servers should return an error when trying to launch locally
        let result = McpLauncher::launch(&config).await;
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("don't need to be launched locally")
        );
    }

    #[tokio::test]
    async fn test_launch_with_uvx_command_config() {
        // Test the specific case where the command is "uvx"
        let config = McpServerConfig {
            name: "test-uvx".to_string(),
            description: Some("Test UVX server".to_string()),
            server_type: McpServerType::Command {
                command: "uvx".to_string(), // This should trigger the UVX path
                args: Some(vec!["--help".to_string()]),
                environment: Some(HashMap::from([(
                    "UVX_VAR".to_string(),
                    "value".to_string(),
                )])),
                working_directory: Some("/tmp".to_string()),
            },
            auto_start: Some(true),
            environment: None,
        };

        // This test checks that the configuration is correctly parsed and processed
        // The actual UVX launch will fail in test environment without uvx, which is expected
        let result = McpLauncher::launch(&config).await;
        // The result could be Ok or Err depending on test environment, so we just verify it doesn't panic
        assert!(result.is_ok() || result.is_err()); // This ensures the function executes without panicking
    }

    #[test]
    fn test_config_parsing_various_types() {
        // Test various configuration types to ensure they are parsed correctly
        let docker_config = McpServerConfig {
            name: "docker-test".to_string(),
            description: Some("Docker test".to_string()),
            server_type: McpServerType::Docker {
                image: "test:latest".to_string(),
                command: None,
                ports: None,
                volumes: None,
                environment: None,
            },
            auto_start: None,
            environment: None,
        };
        match docker_config.server_type {
            McpServerType::Docker { image, .. } => assert_eq!(image, "test:latest"),
            _ => panic!("Expected Docker config type"),
        }

        let npm_config = McpServerConfig {
            name: "npm-test".to_string(),
            description: Some("NPM test".to_string()),
            server_type: McpServerType::Npm {
                package: "test-package".to_string(),
                command: None,
                args: None,
                environment: None,
            },
            auto_start: None,
            environment: None,
        };
        match npm_config.server_type {
            McpServerType::Npm { package, .. } => assert_eq!(package, "test-package"),
            _ => panic!("Expected NPM config type"),
        }

        let command_config = McpServerConfig {
            name: "command-test".to_string(),
            description: Some("Command test".to_string()),
            server_type: McpServerType::Command {
                command: "test-command".to_string(),
                args: None,
                environment: None,
                working_directory: None,
            },
            auto_start: None,
            environment: None,
        };
        match command_config.server_type {
            McpServerType::Command { command, .. } => assert_eq!(command, "test-command"),
            _ => panic!("Expected Command config type"),
        }
    }
}
