//! Tests for session tool permissions functionality.

use OxideAgent::core::session::SessionState;

#[test]
fn test_session_tool_permissions() {
    let mut session = SessionState::new();
    
    // Test that no tools are allowed initially
    assert!(!session.is_tool_allowed("test_tool"));
    
    // Test adding a tool
    session.add_allowed_tool("test_tool".to_string());
    assert!(session.is_tool_allowed("test_tool"));
    
    // Test that adding the same tool again doesn't duplicate
    session.add_allowed_tool("test_tool".to_string());
    assert_eq!(session.list_allowed_tools().len(), 1);
    
    // Test listing allowed tools
    let allowed_tools = session.list_allowed_tools();
    assert_eq!(allowed_tools.len(), 1);
    assert!(allowed_tools.contains(&"test_tool".to_string()));
    
    // Test removing a tool
    assert!(session.remove_allowed_tool("test_tool"));
    assert!(!session.is_tool_allowed("test_tool"));
    assert_eq!(session.list_allowed_tools().len(), 0);
    
    // Test removing a non-existent tool
    assert!(!session.remove_allowed_tool("non_existent_tool"));
}

#[test]
fn test_multiple_session_tools() {
    let mut session = SessionState::new();
    
    // Add multiple tools
    session.add_allowed_tool("tool1".to_string());
    session.add_allowed_tool("tool2".to_string());
    session.add_allowed_tool("tool3".to_string());
    
    // Check all tools are allowed
    assert!(session.is_tool_allowed("tool1"));
    assert!(session.is_tool_allowed("tool2"));
    assert!(session.is_tool_allowed("tool3"));
    assert!(!session.is_tool_allowed("tool4"));
    
    // Check list of tools
    let allowed_tools = session.list_allowed_tools();
    assert_eq!(allowed_tools.len(), 3);
    assert!(allowed_tools.contains(&"tool1".to_string()));
    assert!(allowed_tools.contains(&"tool2".to_string()));
    assert!(allowed_tools.contains(&"tool3".to_string()));
    
    // Remove one tool
    assert!(session.remove_allowed_tool("tool2"));
    assert!(session.is_tool_allowed("tool1"));
    assert!(!session.is_tool_allowed("tool2"));
    assert!(session.is_tool_allowed("tool3"));
    
    // Check updated list
    let allowed_tools = session.list_allowed_tools();
    assert_eq!(allowed_tools.len(), 2);
    assert!(allowed_tools.contains(&"tool1".to_string()));
    assert!(allowed_tools.contains(&"tool3".to_string()));
}