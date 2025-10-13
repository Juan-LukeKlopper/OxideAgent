//! Integration tests for the tool permissions module.

use OxideAgent::core::tool_permissions::GlobalToolPermissions;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_global_tool_permissions_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");

    // Test creating and saving permissions
    let mut permissions = GlobalToolPermissions::new();
    permissions.add_allowed("test_tool");
    permissions.add_allowed("another_tool");

    // Test saving permissions
    let save_result = permissions.save_to_path(&test_file_path);
    assert!(
        save_result.is_ok(),
        "save() failed with error: {:?}",
        save_result.err()
    );

    // Verify file was created
    assert!(test_file_path.exists());

    // Check the content of the file
    let content_result = fs::read_to_string(&test_file_path);
    assert!(
        content_result.is_ok(),
        "Failed to read file: {:?}",
        content_result.err()
    );
    let content = content_result.unwrap();
    assert!(!content.trim().is_empty());

    // Test loading permissions
    let load_result = GlobalToolPermissions::load_from_path(&test_file_path);
    assert!(
        load_result.is_ok(),
        "load() failed with error: {:?}",
        load_result.err()
    );
    let loaded_permissions = load_result.unwrap();
    assert!(loaded_permissions.is_allowed("test_tool"));
    assert!(loaded_permissions.is_allowed("another_tool"));
    assert!(!loaded_permissions.is_allowed("nonexistent_tool"));
}

#[test]
fn test_global_tool_permissions_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");

    // Create and save permissions
    let mut permissions = GlobalToolPermissions::new();
    permissions.add_allowed("tool1");
    permissions.add_allowed("tool2");
    let save_result = permissions.save_to_path(&test_file_path);
    assert!(
        save_result.is_ok(),
        "save() failed with error: {:?}",
        save_result.err()
    );

    // Load in a new instance
    let load_result = GlobalToolPermissions::load_from_path(&test_file_path);
    assert!(
        load_result.is_ok(),
        "load() failed with error: {:?}",
        load_result.err()
    );
    let loaded_permissions = load_result.unwrap();
    assert!(loaded_permissions.is_allowed("tool1"));
    assert!(loaded_permissions.is_allowed("tool2"));
    assert!(!loaded_permissions.is_allowed("tool3"));
}

#[test]
fn test_empty_permissions_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");

    // Create an empty permissions file
    let write_result = fs::write(&test_file_path, "");
    assert!(
        write_result.is_ok(),
        "Failed to write test file: {:?}",
        write_result.err()
    );

    // Loading should still work (return default)
    let load_result = GlobalToolPermissions::load_from_path(&test_file_path);
    assert!(
        load_result.is_ok(),
        "load() failed with error: {:?}",
        load_result.err()
    );
    let permissions = load_result.unwrap();
    assert_eq!(permissions.list_allowed().len(), 0);
}

#[test]
fn test_malformed_permissions_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");

    // Create a malformed permissions file
    let write_result = fs::write(&test_file_path, "{ invalid json }");
    assert!(
        write_result.is_ok(),
        "Failed to write test file: {:?}",
        write_result.err()
    );

    // Loading should still work (return default)
    let load_result = GlobalToolPermissions::load_from_path(&test_file_path);
    assert!(
        load_result.is_ok(),
        "load() failed with error: {:?}",
        load_result.err()
    );
    let permissions = load_result.unwrap();
    assert_eq!(permissions.list_allowed().len(), 0);
}

#[test]
fn test_nonexistent_permissions_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("tool_permissions.json");

    // Make sure no file exists
    if test_file_path.exists() {
        let _ = fs::remove_file(&test_file_path);
    }

    // Loading should still work (return default)
    let load_result = GlobalToolPermissions::load_from_path(&test_file_path);
    assert!(
        load_result.is_ok(),
        "load() failed with error: {:?}",
        load_result.err()
    );
    let permissions = load_result.unwrap();
    assert_eq!(permissions.list_allowed().len(), 0);
}
