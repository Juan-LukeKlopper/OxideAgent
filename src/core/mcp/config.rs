//! MCP configuration management for the OxideAgent system.
//!
//! This module handles parsing and managing MCP server configurations from
//! various file formats including JSON, TOML, and YAML.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Represents the structure of an MCP configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigFile {
    /// Configuration file version
    pub version: String,
    /// List of configured MCP servers
    pub servers: Vec<McpServerConfig>,
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// Unique name for the server
    pub name: String,
    /// Optional description of the server
    pub description: Option<String>,
    /// Server type and connection details
    #[serde(flatten)]
    pub server_type: McpServerType,
    /// Whether to automatically start this server
    #[serde(rename = "autoStart")]
    pub auto_start: Option<bool>,
    /// Environment variables to set for the server
    pub environment: Option<HashMap<String, String>>,
}

/// Different types of MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpServerType {
    /// Remote MCP server accessible via HTTP
    #[serde(rename = "remote")]
    Remote {
        /// Server URL
        url: String,
        /// Access token for authentication
        #[serde(rename = "accessToken")]
        access_token: Option<String>,
        /// API key for authentication (alternative to access token)
        #[serde(rename = "apiKey")]
        api_key: Option<String>,
    },
    /// Docker-based MCP server
    #[serde(rename = "docker")]
    Docker {
        /// Docker image to use
        image: String,
        /// Command to run in the container
        command: Option<Vec<String>>,
        /// Port mappings (e.g., "3000:3000")
        ports: Option<Vec<String>>,
        /// Volume mappings (e.g., "/host/path:/container/path")
        volumes: Option<Vec<String>>,
        /// Environment variables for the container
        environment: Option<HashMap<String, String>>,
    },
    /// NPM-based MCP server
    #[serde(rename = "npm")]
    Npm {
        /// NPM package name
        package: String,
        /// Command to run (default: "start")
        command: Option<String>,
        /// Arguments to pass to the command
        args: Option<Vec<String>>,
        /// Environment variables for the process
        environment: Option<HashMap<String, String>>,
    },
    /// Command-based MCP server
    #[serde(rename = "command")]
    Command {
        /// Command to execute
        command: String,
        /// Arguments to pass to the command
        args: Option<Vec<String>>,
        /// Environment variables for the process
        environment: Option<HashMap<String, String>>,
        /// Working directory for the command
        #[serde(rename = "workingDirectory")]
        working_directory: Option<String>,
    },
}

impl McpConfigFile {
    /// Load configuration from a file, automatically detecting the format based on extension
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("json");
        
        match extension {
            "json" => Ok(serde_json::from_str(&content)?),
            "toml" => Ok(toml::from_str(&content)?),
            "yaml" | "yml" => Ok(serde_yaml::from_str(&content)?),
            _ => Err(anyhow::anyhow!("Unsupported configuration file format: {}", extension)),
        }
    }
    
    /// Save configuration to a file, automatically detecting the format based on extension
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        let content = match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => serde_json::to_string_pretty(self)?,
            Some("toml") => toml::to_string_pretty(self)?,
            Some("yaml") | Some("yml") => serde_yaml::to_string(self)?,
            _ => serde_json::to_string_pretty(self)?,
        };
        
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Create a new empty configuration file
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            servers: Vec::new(),
        }
    }
    
    /// Get a server configuration by name
    pub fn get_server(&self, name: &str) -> Option<&McpServerConfig> {
        self.servers.iter().find(|server| server.name == name)
    }
    
    /// Add a new server configuration
    pub fn add_server(&mut self, server: McpServerConfig) {
        self.servers.push(server);
    }
    
    /// Remove a server configuration by name
    pub fn remove_server(&mut self, name: &str) -> bool {
        let initial_len = self.servers.len();
        self.servers.retain(|server| server.name != name);
        self.servers.len() < initial_len
    }
    
    /// List all server names
    pub fn list_server_names(&self) -> Vec<String> {
        self.servers.iter().map(|server| server.name.clone()).collect()
    }
}

impl Default for McpConfigFile {
    fn default() -> Self {
        Self::new()
    }
}