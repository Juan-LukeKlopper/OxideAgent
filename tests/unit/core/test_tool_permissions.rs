//! Tests for the tool permissions module.

use OxideAgent::core::tool_permissions::GlobalToolPermissions;

#[test]
fn test_global_tool_permissions() {
    // Test creating new permissions
    let mut permissions = GlobalToolPermissions::new();
    assert!(!permissions.is_allowed("test_tool"));
    
    // Test adding a tool
    permissions.add_allowed("test_tool");
    assert!(permissions.is_allowed("test_tool"));
    
    // Test listing allowed tools
    let allowed_tools = permissions.list_allowed();
    assert_eq!(allowed_tools.len(), 1);
    assert!(allowed_tools.contains(&"test_tool".to_string()));
    
    // Test removing a tool
    assert!(permissions.remove_allowed("test_tool"));
    assert!(!permissions.is_allowed("test_tool"));
    assert!(!permissions.remove_allowed("non_existent_tool"));
}

#[test]
fn test_global_tool_permissions_default() {
    // Test default permissions
    let permissions = GlobalToolPermissions::default();
    assert_eq!(permissions.list_allowed().len(), 0);
    assert!(!permissions.is_allowed("any_tool"));
}

#[test]
fn test_global_tool_permissions_duplicate_add() {
    // Test that adding the same tool twice doesn't create duplicates
    let mut permissions = GlobalToolPermissions::new();
    permissions.add_allowed("test_tool");
    permissions.add_allowed("test_tool");
    assert_eq!(permissions.list_allowed().len(), 1);
    assert!(permissions.is_allowed("test_tool"));
}

#[test]
fn test_global_tool_permissions_remove_nonexistent() {
    // Test removing a tool that doesn't exist
    let mut permissions = GlobalToolPermissions::new();
    assert!(!permissions.remove_allowed("nonexistent_tool"));
}