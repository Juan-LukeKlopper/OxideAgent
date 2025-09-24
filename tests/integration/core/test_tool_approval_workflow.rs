//! Integration tests for the tool approval workflow.

use OxideAgent::core::tool_permissions::GlobalToolPermissions;
use OxideAgent::core::session::SessionState;
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_tool_approval_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");
    
    // Make sure no existing file interferes
    let _ = fs::remove_file(&test_file_path);
    
    // Test global tool permissions workflow
    let mut global_permissions = GlobalToolPermissions::new();
    assert!(!global_permissions.is_allowed("test_tool"));
    
    // Add a tool to global permissions
    global_permissions.add_allowed("test_tool");
    assert!(global_permissions.is_allowed("test_tool"));
    
    // Save and reload
    assert!(global_permissions.save_to_path(&test_file_path).is_ok());
    let loaded_permissions = GlobalToolPermissions::load_from_path(&test_file_path).unwrap();
    assert!(loaded_permissions.is_allowed("test_tool"), "Expected test_tool to be allowed after loading, but it wasn't. Loaded permissions: {:?}", loaded_permissions.list_allowed());
    
    // Test session tool permissions workflow
    let mut session_state = SessionState::new();
    assert!(!session_state.is_tool_allowed("session_tool"));
    
    // Add a tool to session permissions
    session_state.add_allowed_tool("session_tool".to_string());
    assert!(session_state.is_tool_allowed("session_tool"));
    
    // List allowed tools
    let allowed_tools = session_state.list_allowed_tools();
    assert_eq!(allowed_tools.len(), 1);
    assert!(allowed_tools.contains(&"session_tool".to_string()));
    
    // Remove a tool
    assert!(session_state.remove_allowed_tool("session_tool"));
    assert!(!session_state.is_tool_allowed("session_tool"));
    assert!(!session_state.remove_allowed_tool("non_existent_tool"));
}

#[test]
fn test_tool_permissions_isolation() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    let test_file_path1 = temp_dir1.path().join("tool_permissions.json");
    let test_file_path2 = temp_dir2.path().join("tool_permissions.json");
    
    // Test in directory 1
    let mut permissions1 = GlobalToolPermissions::new();
    permissions1.add_allowed("tool1");
    permissions1.save_to_path(&test_file_path1).unwrap();
    
    // Test in directory 2
    let mut permissions2 = GlobalToolPermissions::new();
    permissions2.add_allowed("tool2");
    permissions2.save_to_path(&test_file_path2).unwrap();
    
    // Verify isolation - load permissions from temp_dir1
    let loaded1 = GlobalToolPermissions::load_from_path(&test_file_path1).unwrap();
    assert!(!loaded1.is_allowed("tool2")); // This should be false, as tool2 was added in directory 2
    
    // Also verify that tool1 is still there
    assert!(loaded1.is_allowed("tool1"));
}

#[test]
fn test_empty_tool_permissions_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");
    
    // Create an empty permissions file
    fs::write(&test_file_path, "").unwrap();
    
    // Loading should still work (return default)
    let permissions = GlobalToolPermissions::load_from_path(&test_file_path).unwrap();
    assert_eq!(permissions.list_allowed().len(), 0);
}