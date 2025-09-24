//! Simple test to verify tool permissions work correctly in isolation.

use OxideAgent::core::tool_permissions::GlobalToolPermissions;
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_simple_tool_permissions() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");
    
    // Make sure no existing file interferes
    if test_file_path.exists() {
        let _ = fs::remove_file(&test_file_path);
    }
    
    // Test creating new permissions
    let mut permissions = GlobalToolPermissions::new();
    assert!(!permissions.is_allowed("test_tool"));
    
    // Add a tool
    permissions.add_allowed("test_tool");
    assert!(permissions.is_allowed("test_tool"));
    
    // Save permissions
    assert!(permissions.save_to_path(&test_file_path).is_ok());
    
    // Load permissions
    let loaded_permissions = GlobalToolPermissions::load_from_path(&test_file_path).unwrap();
    assert!(loaded_permissions.is_allowed("test_tool"));
    
    // Test listing allowed tools
    let allowed_tools = loaded_permissions.list_allowed();
    assert_eq!(allowed_tools.len(), 1);
    assert!(allowed_tools.contains(&"test_tool".to_string()));
    
    // Test removing tools
    let mut permissions_clone = loaded_permissions.clone();
    assert!(permissions_clone.remove_allowed("test_tool"));
    assert!(!permissions_clone.is_allowed("test_tool"));
    
    // Test that removing non-existent tool returns false
    assert!(!permissions_clone.remove_allowed("non_existent_tool"));
}

#[test]
fn test_simple_tool_permissions_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");
    
    // Test duplicate additions
    let mut permissions = GlobalToolPermissions::new();
    permissions.add_allowed("duplicate_tool");
    permissions.add_allowed("duplicate_tool"); // Add again
    
    // Should still only have one instance
    let allowed_tools = permissions.list_allowed();
    assert_eq!(allowed_tools.len(), 1);
    assert!(allowed_tools.contains(&"duplicate_tool".to_string()));
    
    // Test saving and loading empty permissions
    assert!(permissions.save_to_path(&test_file_path).is_ok());
    let loaded_permissions = GlobalToolPermissions::load_from_path(&test_file_path).unwrap();
    
    // Add another tool to verify we can extend existing permissions
    let mut extended_permissions = loaded_permissions;
    extended_permissions.add_allowed("another_tool");
    
    let allowed_tools = extended_permissions.list_allowed();
    assert_eq!(allowed_tools.len(), 2);
    assert!(allowed_tools.contains(&"duplicate_tool".to_string()));
    assert!(allowed_tools.contains(&"another_tool".to_string()));
}