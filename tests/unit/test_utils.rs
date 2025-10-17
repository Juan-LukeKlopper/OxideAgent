//! Tests for mock objects and test utilities.

// We can't directly import from the test utilities in integration tests
// So we'll just test the functionality that doesn't require the mock objects

use OxideAgent::config::{AgentConfig, AgentType, InterfaceType, OxideConfig as Config};
use OxideAgent::core::tools::{Tool, ToolProfile};
use async_trait::async_trait;
use serde_json::json;

#[tokio::test]
async fn test_mock_tool_success() {
    // Since we can't import the mock tool directly, we'll create a simple test tool inline
    struct TestTool {
        should_fail: bool,
    }

    #[async_trait]
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

        async fn execute(&self, _args: &serde_json::Value) -> anyhow::Result<String> {
            if self.should_fail {
                Err(anyhow::anyhow!("Mock tool failed as requested"))
            } else {
                Ok("Success result".to_string())
            }
        }

        fn clone_box(&self) -> Box<dyn Tool> {
            Box::new(TestTool {
                should_fail: self.should_fail,
            })
        }
    }

    let tool = TestTool { should_fail: false };

    assert_eq!(tool.name(), "test_tool");
    assert_eq!(tool.description(), "A test tool");
    assert_eq!(tool.profile(), ToolProfile::Generic);

    let result = tool.execute(&json!({})).await.unwrap();
    assert_eq!(result, "Success result");
}

#[tokio::test]
async fn test_mock_tool_failure() {
    struct TestTool {
        should_fail: bool,
    }

    #[async_trait]
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

        async fn execute(&self, _args: &serde_json::Value) -> anyhow::Result<String> {
            if self.should_fail {
                Err(anyhow::anyhow!("Mock tool failed as requested"))
            } else {
                Ok("Success result".to_string())
            }
        }

        fn clone_box(&self) -> Box<dyn Tool> {
            Box::new(TestTool {
                should_fail: self.should_fail,
            })
        }
    }

    let tool = TestTool { should_fail: true };

    let result = tool.execute(&json!({})).await;
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
            model: "qwen:latest".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: false,
        session: Some("test_session".to_string()),
        list_sessions: false,
        interface: InterfaceType::Tui,
        mcp: OxideAgent::config::MCPConfig {
            server: None,
            auth_token: None,
            tools: vec![],
        },
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: "http://localhost:11434".to_string(),
            api_key: None,
            model: None,
        },
    };

    assert_eq!(config.agent.name, "Qwen");
    assert_eq!(config.agent.model, "qwen:latest");
    assert_eq!(config.session, Some("test_session".to_string()));
}
