//! Tool permissions management for the OxideAgent system.
//!
//! This module handles persistent storage and management of tool permissions,
//! including global permissions and session-specific permissions.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Global mutex to synchronize file access for tool permissions
static TOOL_PERMISSIONS_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Global tool permissions that apply across all sessions
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GlobalToolPermissions {
    /// Set of tools that are always allowed
    #[serde(default)]
    allowed_tools: HashSet<String>,
}

impl GlobalToolPermissions {
    /// Create a new empty global tool permissions instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Load global tool permissions from the default file
    pub fn load() -> anyhow::Result<Self> {
        Self::load_from_path("tool_permissions.json")
    }

    /// Load global tool permissions from a specific file path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        // Acquire lock to prevent race conditions during file access
        let _lock = TOOL_PERMISSIONS_MUTEX.lock().unwrap();
        
        let path = path.as_ref();
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    eprintln!("DEBUG: Reading content from file: '{}'", content);
                    // Handle empty file
                    if content.trim().is_empty() {
                        eprintln!("DEBUG: Empty file, returning default");
                        return Ok(Self::default());
                    }
                    match serde_json::from_str(&content) {
                        Ok(permissions) => {
                            eprintln!("DEBUG: Successfully parsed permissions: {:?}", permissions);
                            Ok(permissions)
                        },
                        Err(e) => {
                            // Log error but don't crash
                            eprintln!("Warning: Failed to parse tool permissions file '{}': {}", path.display(), e);
                            eprintln!("Using default tool permissions as fallback.");
                            Ok(Self::default())
                        }
                    }
                }
                Err(e) => {
                    // Log error but don't crash
                    eprintln!("Warning: Failed to read tool permissions file '{}': {}", path.display(), e);
                    Ok(Self::default())
                }
            }
        } else {
            eprintln!("DEBUG: File doesn't exist, returning default");
            Ok(Self::default())
        }
    }

    /// Save global tool permissions to the default file
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to_path("tool_permissions.json")
    }

    /// Save global tool permissions to a specific file path
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        // Acquire lock to prevent race conditions during file access
        let _lock = TOOL_PERMISSIONS_MUTEX.lock().unwrap();
        
        let content = serde_json::to_string_pretty(self)?;

        // Ensure the directory exists before writing
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use atomic write to prevent corruption from concurrent writes
        let temp_path = path.with_extension(format!("tmp.{}", std::process::id()));
        std::fs::write(&temp_path, content.as_bytes())?;
        std::fs::rename(&temp_path, path)?;
        Ok(())
    }

    /// Check if a tool is allowed globally
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        self.allowed_tools.contains(tool_name)
    }

    /// Add a tool to the global allowed list
    pub fn add_allowed(&mut self, tool_name: &str) {
        self.allowed_tools.insert(tool_name.to_string());
    }

    /// Remove a tool from the global allowed list
    pub fn remove_allowed(&mut self, tool_name: &str) -> bool {
        self.allowed_tools.remove(tool_name)
    }

    /// List all globally allowed tools
    pub fn list_allowed(&self) -> Vec<String> {
        self.allowed_tools.iter().cloned().collect()
    }
}

