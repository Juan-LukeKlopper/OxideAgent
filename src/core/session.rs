//! Session management for the OxideAgent system.
//!
//! This module handles session persistence, loading, saving, and listing.

use crate::types::ChatMessage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionState {
    #[serde(default)]
    history: Vec<ChatMessage>,
    /// Tools that are allowed for this specific session
    #[serde(default)] // Add default to handle missing field in existing files
    allowed_tools: Vec<String>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            allowed_tools: Vec::new(), // Explicitly initialize as empty
        }
    }

    pub fn history(&self) -> &Vec<ChatMessage> {
        &self.history
    }

    pub fn set_history(&mut self, history: Vec<ChatMessage>) {
        self.history = history;
    }

    /// Check if a tool is allowed for this session
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.allowed_tools.contains(&tool_name.to_string())
    }

    /// Add a tool to the session allowed list
    pub fn add_allowed_tool(&mut self, tool_name: String) {
        if !self.allowed_tools.contains(&tool_name) {
            self.allowed_tools.push(tool_name);
        }
    }

    /// Remove a tool from the session allowed list
    pub fn remove_allowed_tool(&mut self, tool_name: &str) -> bool {
        let initial_len = self.allowed_tools.len();
        self.allowed_tools.retain(|tool| tool != tool_name);
        self.allowed_tools.len() < initial_len
    }

    /// List all tools allowed for this session
    pub fn list_allowed_tools(&self) -> Vec<String> {
        self.allowed_tools.clone()
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SessionManager;

impl SessionManager {
    /// Load session state from a file
    pub fn load_state<P: AsRef<Path>>(path: P) -> anyhow::Result<Option<SessionState>> {
        let path = path.as_ref();
        if path.exists() {
            // Retry mechanism for handling temporary file access issues during race conditions
            let mut attempts = 0;
            let max_attempts = 3;

            loop {
                match fs::read_to_string(path) {
                    Ok(content) => {
                        if content.trim().is_empty() {
                            // Empty file, return default session state
                            return Ok(Some(SessionState::new()));
                        } else {
                            // Try to parse the content
                            match serde_json::from_str(&content) {
                                Ok(session_state) => return Ok(Some(session_state)),
                                Err(e) => {
                                    // Log the error but don't crash
                                    eprintln!(
                                        "Warning: Failed to parse session file '{}': {}",
                                        path.display(),
                                        e
                                    );
                                    eprintln!("Creating new session state as fallback.");
                                    return Ok(Some(SessionState::new()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // If it's a file access error and we haven't reached max attempts, retry
                        if (e.kind() == std::io::ErrorKind::NotFound
                            || e.kind() == std::io::ErrorKind::PermissionDenied)
                            && attempts < max_attempts - 1
                        {
                            attempts += 1;
                            std::thread::sleep(std::time::Duration::from_millis(10 * attempts));
                        } else {
                            // Log the error but don't crash
                            eprintln!(
                                "Warning: Failed to read session file '{}': {}",
                                path.display(),
                                e
                            );
                            return Ok(Some(SessionState::new()));
                        }
                    }
                }
            }
        } else {
            // File doesn't exist, return None
            Ok(None)
        }
    }

    /// Save session state to a file
    pub fn save_state<P: AsRef<Path>>(path: P, session_state: &SessionState) -> anyhow::Result<()> {
        let path = path.as_ref();
        let content = serde_json::to_string_pretty(session_state)?;

        // Ensure the directory exists with retry mechanism
        if let Some(parent) = path.parent() {
            let mut attempts = 0;
            let max_attempts = 3;

            loop {
                match fs::create_dir_all(parent) {
                    Ok(_) => break,
                    Err(e) => {
                        if attempts >= max_attempts - 1 {
                            return Err(e.into());
                        } else {
                            attempts += 1;
                            std::thread::sleep(std::time::Duration::from_millis(10 * attempts));
                        }
                    }
                }
            }
        }

        // Use atomic write with retry mechanism
        let temp_path = path.with_extension(format!("tmp.{}", std::process::id()));
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            match fs::write(&temp_path, content.as_bytes()) {
                Ok(_) => {
                    match fs::rename(&temp_path, path) {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            if attempts >= max_attempts - 1 {
                                fs::remove_file(&temp_path).ok(); // Clean up temp file
                                return Err(e.into());
                            } else {
                                attempts += 1;
                                std::thread::sleep(std::time::Duration::from_millis(10 * attempts));
                            }
                        }
                    }
                }
                Err(e) => {
                    if attempts >= max_attempts - 1 {
                        fs::remove_file(&temp_path).ok(); // Clean up temp file if it exists
                        return Err(e.into());
                    } else {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(10 * attempts));
                    }
                }
            }
        }
    }

    /// List all available sessions
    pub fn list_sessions() -> anyhow::Result<Vec<String>> {
        let mut sessions = Vec::new();

        // Check for the default session file
        if Path::new("session.json").exists() {
            sessions.push("default".to_string());
        }

        // Look for named session files
        let entries = fs::read_dir(".")?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.starts_with("session_") && file_name_str.ends_with(".json")
                        {
                            // Extract session name from file name (remove "session_" prefix and ".json" suffix)
                            let session_name = file_name_str
                                .strip_prefix("session_")
                                .unwrap()
                                .strip_suffix(".json")
                                .unwrap();
                            sessions.push(session_name.to_string());
                        }
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Get the session filename for a given session name
    pub fn get_session_filename(session_name: Option<&str>) -> String {
        match session_name {
            Some(name) => format!("session_{}.json", name),
            None => "session.json".to_string(),
        }
    }
}
