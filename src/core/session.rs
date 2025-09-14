//! Session management for the OxideAgent system.
//!
//! This module handles session persistence, loading, saving, and listing.

use crate::types::ChatMessage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionState {
    history: Vec<ChatMessage>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn history(&self) -> &Vec<ChatMessage> {
        &self.history
    }

    pub fn set_history(&mut self, history: Vec<ChatMessage>) {
        self.history = history;
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SessionManager;

impl SessionManager {
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

            if path.is_file()
                && let Some(file_name) = path.file_name()
                && let Some(file_name_str) = file_name.to_str()
                && file_name_str.starts_with("session_") && file_name_str.ends_with(".json")
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

        Ok(sessions)
    }

    pub fn load_state(session_file: &str) -> anyhow::Result<Option<SessionState>> {
        if Path::new(session_file).exists() {
            let session_json = fs::read_to_string(session_file)?;
            if !session_json.trim().is_empty() {
                let session_state: SessionState = serde_json::from_str(&session_json)?;
                return Ok(Some(session_state));
            }
        }
        Ok(None)
    }

    pub fn save_state(session_file: &str, session_state: &SessionState) -> anyhow::Result<()> {
        let session_json = serde_json::to_string_pretty(session_state)?;
        fs::write(session_file, session_json)?;
        Ok(())
    }

    pub fn get_session_filename(session_name: Option<&str>) -> String {
        match session_name {
            Some(name) => format!("session_{}.json", name),
            None => "session.json".to_string(),
        }
    }
}
