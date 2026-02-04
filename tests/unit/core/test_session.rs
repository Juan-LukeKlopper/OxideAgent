//! Unit tests for the session module.

use OxideAgent::core::session::{SessionManager, SessionState};
use OxideAgent::types::ChatMessage;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_session_state_new() {
    let session_state = SessionState::new();

    assert_eq!(session_state.history().len(), 0);
    assert_eq!(session_state.list_allowed_tools().len(), 0);
}

#[test]
fn test_session_state_history() {
    let mut session_state = SessionState::new();

    // Add a message
    let message = ChatMessage::user("Test message");
    session_state.set_history(vec![message.clone()]);

    // Check history
    assert_eq!(session_state.history().len(), 1);
    assert_eq!(session_state.history()[0].content, "Test message");
}

#[test]
fn test_session_state_tool_permissions() {
    let mut session_state = SessionState::new();

    // Initially no tools allowed
    assert!(!session_state.is_tool_allowed("test_tool"));

    // Add a tool
    session_state.add_allowed_tool("test_tool".to_string());
    assert!(session_state.is_tool_allowed("test_tool"));

    // Add another tool
    session_state.add_allowed_tool("another_tool".to_string());
    assert!(session_state.is_tool_allowed("test_tool"));
    assert!(session_state.is_tool_allowed("another_tool"));

    // List allowed tools
    let allowed_tools = session_state.list_allowed_tools();
    assert_eq!(allowed_tools.len(), 2);
    assert!(allowed_tools.contains(&"test_tool".to_string()));
    assert!(allowed_tools.contains(&"another_tool".to_string()));

    // Remove a tool
    assert!(session_state.remove_allowed_tool("test_tool"));
    assert!(!session_state.is_tool_allowed("test_tool"));
    assert!(session_state.is_tool_allowed("another_tool"));

    // Try to remove non-existent tool
    assert!(!session_state.remove_allowed_tool("nonexistent_tool"));
}

#[test]
fn test_session_state_tool_permissions_duplicate() {
    let mut session_state = SessionState::new();

    // Add the same tool twice
    session_state.add_allowed_tool("test_tool".to_string());
    session_state.add_allowed_tool("test_tool".to_string());

    // Should only appear once
    let allowed_tools = session_state.list_allowed_tools();
    assert_eq!(allowed_tools.len(), 1);
    assert!(allowed_tools.contains(&"test_tool".to_string()));
}

#[test]
fn test_session_manager_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("temp_test_session.json");

    // Create a session state
    let mut session_state = SessionState::new();
    session_state.set_history(vec![ChatMessage::user("Test message")]);
    session_state.add_allowed_tool("test_tool".to_string());

    // Save the session state
    let save_result = SessionManager::save_state(&temp_file, &session_state);
    assert!(
        save_result.is_ok(),
        "Failed to save state: {:?}",
        save_result.err()
    );

    // Verify the file exists
    assert!(temp_file.exists());

    // Load the session state
    let load_result = SessionManager::load_state(&temp_file);
    assert!(
        load_result.is_ok(),
        "Failed to load state: {:?}",
        load_result.err()
    );

    let loaded_state = load_result.unwrap();
    assert!(loaded_state.is_some());
    let loaded_state = loaded_state.unwrap();

    // Verify the loaded state matches the saved state
    assert_eq!(loaded_state.history().len(), 1);
    assert_eq!(loaded_state.list_allowed_tools().len(), 1);
    assert!(loaded_state.is_tool_allowed("test_tool"));
}

#[test]
fn test_session_manager_load_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_file = temp_dir.path().join("nonexistent_session.json");

    // Try to load a non-existent session
    let result = SessionManager::load_state(&nonexistent_file);
    assert!(
        result.is_ok(),
        "Loading nonexistent should succeed with None: {:?}",
        result.err()
    );

    let result = result.unwrap();
    assert!(result.is_none());
}

#[test]
fn test_session_manager_load_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("temp_empty_session.json");

    // Create an empty file
    let write_result = fs::write(&temp_file, "");
    assert!(
        write_result.is_ok(),
        "Failed to write test file: {:?}",
        write_result.err()
    );

    // Try to load from the empty file
    let result = SessionManager::load_state(&temp_file);
    assert!(
        result.is_ok(),
        "Failed to load empty file: {:?}",
        result.err()
    );

    let session_state = result.unwrap();
    assert!(session_state.is_some());
    let session_state = session_state.unwrap();

    // Should have default empty state
    assert_eq!(session_state.history().len(), 0);
    assert_eq!(session_state.list_allowed_tools().len(), 0);
}

#[test]
fn test_session_manager_load_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("temp_invalid_session.json");

    // Create a file with invalid JSON
    let write_result = fs::write(&temp_file, "{ invalid json");
    assert!(
        write_result.is_ok(),
        "Failed to write test file: {:?}",
        write_result.err()
    );

    // Try to load from the invalid file
    // This should not panic and should return a default session state
    let result = SessionManager::load_state(&temp_file);
    assert!(
        result.is_ok(),
        "Failed to load invalid JSON: {:?}",
        result.err()
    );

    let session_state = result.unwrap();
    assert!(session_state.is_some());
    let session_state = session_state.unwrap();

    // Should have default empty state despite invalid JSON
    assert_eq!(session_state.history().len(), 0);
    assert_eq!(session_state.list_allowed_tools().len(), 0);
}

use lazy_static::lazy_static;
use std::path::PathBuf;
use std::sync::Mutex;

lazy_static! {
    static ref CWD_MUTEX: Mutex<()> = Mutex::new(());
}

struct CwdGuard {
    original: PathBuf,
}

impl CwdGuard {
    fn new() -> Self {
        let original = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self { original }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

#[test]
fn test_session_manager_list_sessions_default() {
    let _lock = CWD_MUTEX.lock().unwrap();
    let _cwd_guard = CwdGuard::new();
    let temp_dir = TempDir::new().unwrap();
    // let original_cwd = std::env::current_dir().unwrap(); // Handled by guard

    // Change to temp directory for testing
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create a default session file
    let default_session = "session.json";
    let session_state = SessionState::new();
    let save_result = SessionManager::save_state(default_session, &session_state);
    assert!(
        save_result.is_ok(),
        "Failed to save default session: {:?}",
        save_result.err()
    );

    // List sessions
    let sessions_result = SessionManager::list_sessions();
    assert!(
        sessions_result.is_ok(),
        "Failed to list sessions: {:?}",
        sessions_result.err()
    );

    let sessions = sessions_result.unwrap();

    // Check if default session is listed
    assert!(sessions.contains(&"default".to_string()));
}

#[test]
fn test_session_manager_list_sessions_named() {
    let _lock = CWD_MUTEX.lock().unwrap();
    let _cwd_guard = CwdGuard::new();
    let temp_dir = TempDir::new().unwrap();

    // Change to temp directory for testing
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create a named session file
    let named_session = "session_test_named.json";
    let session_state = SessionState::new();
    let save_result = SessionManager::save_state(named_session, &session_state);
    assert!(
        save_result.is_ok(),
        "Failed to save named session: {:?}",
        save_result.err()
    );

    // List sessions
    let sessions_result = SessionManager::list_sessions();
    assert!(
        sessions_result.is_ok(),
        "Failed to list sessions: {:?}",
        sessions_result.err()
    );

    let sessions = sessions_result.unwrap();

    // Check if named session is listed
    assert!(sessions.contains(&"test_named".to_string()));
}

#[test]
fn test_session_manager_get_session_filename() {
    // Test default session filename
    assert_eq!(SessionManager::get_session_filename(None), "session.json");

    // Test named session filename
    assert_eq!(
        SessionManager::get_session_filename(Some("test_session")),
        "session_test_session.json"
    );
}

#[test]
fn test_session_manager_list_sessions_sorted_by_recency() {
    let _lock = CWD_MUTEX.lock().unwrap();
    let _cwd_guard = CwdGuard::new();
    let temp_dir = TempDir::new().unwrap();

    // Change to temp directory for testing
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create sessions with delays to ensure different mtimes
    let session_state = SessionState::new();

    // Create "old" session first
    SessionManager::save_state("session_old.json", &session_state).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Create "middle" session
    SessionManager::save_state("session_middle.json", &session_state).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Create "new" session last (most recent)
    SessionManager::save_state("session_new.json", &session_state).unwrap();

    // List sessions - should be sorted by recency (newest first)
    let sessions = SessionManager::list_sessions().unwrap();

    // Find positions
    let new_pos = sessions.iter().position(|s| s == "new");
    let middle_pos = sessions.iter().position(|s| s == "middle");
    let old_pos = sessions.iter().position(|s| s == "old");

    // Verify ordering: new < middle < old (newer sessions have lower indices)
    assert!(new_pos.is_some(), "new session not found");
    assert!(middle_pos.is_some(), "middle session not found");
    assert!(old_pos.is_some(), "old session not found");

    assert!(
        new_pos.unwrap() < middle_pos.unwrap(),
        "new session should appear before middle session"
    );
    assert!(
        middle_pos.unwrap() < old_pos.unwrap(),
        "middle session should appear before old session"
    );
}
