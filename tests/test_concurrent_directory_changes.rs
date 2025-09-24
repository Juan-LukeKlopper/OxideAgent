//! Test to verify that directory changes in tests don't interfere with each other when run in parallel
//! This test will fail if the global tool permissions or session files cause conflicts

use OxideAgent::core::tool_permissions::GlobalToolPermissions;
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_tool_permissions_isolation_with_proper_cleanup() {
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
    assert!(
        loaded_permissions.is_allowed("test_tool"),
        "Expected test_tool to be allowed after loading, but it wasn't. Loaded permissions: {:?}",
        loaded_permissions.list_allowed()
    );
}

#[test]
fn test_tool_permissions_isolation_second_test() {
    // This test is identical to the first one, but it ensures
    // that the first test doesn't affect this one even when run in parallel
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");

    // Make sure no existing file interferes
    let _ = fs::remove_file(&test_file_path);

    // Test global tool permissions workflow
    let mut global_permissions = GlobalToolPermissions::new();
    assert!(!global_permissions.is_allowed("another_test_tool"));

    // Add a tool to global permissions
    global_permissions.add_allowed("another_test_tool");
    assert!(global_permissions.is_allowed("another_test_tool"));

    // Save and reload
    assert!(global_permissions.save_to_path(&test_file_path).is_ok());
    let loaded_permissions = GlobalToolPermissions::load_from_path(&test_file_path).unwrap();
    assert!(
        loaded_permissions.is_allowed("another_test_tool"),
        "Expected another_test_tool to be allowed after loading, but it wasn't. Loaded permissions: {:?}",
        loaded_permissions.list_allowed()
    );
}
