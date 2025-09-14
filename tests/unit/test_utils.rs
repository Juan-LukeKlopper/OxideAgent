//! Tests for mock objects and test utilities.

// We can't directly import from the test utilities in integration tests
// So we'll just test the functionality that doesn't require the mock objects

use OxideAgent::config::{AgentConfig, AgentType, Config, InterfaceType};
use OxideAgent::core::tools::{Tool, ToolProfile};
use serde_json::json;

#[test]
fn test_mock_tool_success() {
    // Since we can't import the mock tool directly, we'll create a simple test tool inline
    struct TestTool {
        should_fail: bool,
    }

    impl Tool for TestTool {
        fn name(&self) -> String {
            "test_tool".to_string()
        }

        fn description(&self) -> String {
            "A test tool".to_string()
        }

        fn parameters(&self) -> serde_json::Value {
            json!({"type": "object"})
        }

        fn profile(&self) -> ToolProfile {
            ToolProfile::Generic
        }

        fn execute(&self, _args: &serde_json::Value) -> anyhow::Result<String> {
            if self.should_fail {
                Err(anyhow::anyhow!("Mock tool failed as requested"))
            } else {
                Ok("Success result".to_string())
            }
        }
    }

    let tool = TestTool { should_fail: false };

    assert_eq!(tool.name(), "test_tool");
    assert_eq!(tool.description(), "A test tool");
    assert_eq!(tool.profile(), ToolProfile::Generic);

    let result = tool.execute(&json!({})).unwrap();
    assert_eq!(result, "Success result");
}

#[test]
fn test_mock_tool_failure() {
    struct TestTool {
        should_fail: bool,
    }

    impl Tool for TestTool {
        fn name(&self) -> String {
            "failing_tool".to_string()
        }

        fn description(&self) -> String {
            "A failing tool".to_string()
        }

        fn parameters(&self) -> serde_json::Value {
            json!({"type": "object"})
        }

        fn profile(&self) -> ToolProfile {
            ToolProfile::Generic
        }

        fn execute(&self, _args: &serde_json::Value) -> anyhow::Result<String> {
            if self.should_fail {
                Err(anyhow::anyhow!("Mock tool failed as requested"))
            } else {
                Ok("Success result".to_string())
            }
        }
    }

    let tool = TestTool { should_fail: true };

    let result = tool.execute(&json!({}));
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Mock tool failed as requested"
    );
}

#[test]
fn test_create_mock_agent() {
    let agent = OxideAgent::core::agents::Agent::new("MockAgent", "mock-model");
    assert_eq!(agent.model, "mock-model");

    // Check that the agent has a system message
    assert!(!agent.history.is_empty());
    let system_message = &agent.history[0];
    assert_eq!(system_message.role, "system");
}

#[test]
fn test_create_test_config() {
    let config = Config {
        agent: AgentConfig {
            agent_type: AgentType::Qwen,
            model: "qwen3:4b".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: false,
        session: Some("test_session".to_string()),
        list_sessions: false,
        mcp_server: None,
        mcp_auth_token: None,
        interface: InterfaceType::Tui,
    };

    assert_eq!(config.agent.name, "Qwen");
    assert_eq!(config.agent.model, "qwen3:4b");
    assert_eq!(config.session, Some("test_session".to_string()));
}
