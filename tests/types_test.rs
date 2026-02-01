use OxideAgent::types::{
    AppEvent, ChatMessage, Tool, ToolApprovalResponse, ToolCall, ToolFunction,
};
use serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_approval_response_enum() {
        // Test all variants of ToolApprovalResponse
        assert_eq!(format!("{:?}", ToolApprovalResponse::Allow), "Allow");
        assert_eq!(
            format!("{:?}", ToolApprovalResponse::AlwaysAllow),
            "AlwaysAllow"
        );
        assert_eq!(
            format!("{:?}", ToolApprovalResponse::AlwaysAllowSession),
            "AlwaysAllowSession"
        );
        assert_eq!(format!("{:?}", ToolApprovalResponse::Deny), "Deny");

        // Test that all variants can be created
        let _allow = ToolApprovalResponse::Allow;
        let _always_allow = ToolApprovalResponse::AlwaysAllow;
        let _always_allow_session = ToolApprovalResponse::AlwaysAllowSession;
        let _deny = ToolApprovalResponse::Deny;

        // Test clone functionality
        let original = ToolApprovalResponse::Allow;
        let cloned = original.clone();
        assert!(matches!(cloned, ToolApprovalResponse::Allow));
    }

    #[test]
    fn test_app_event_enum() {
        // Test all variants of AppEvent
        assert_eq!(
            format!("{:?}", AppEvent::UserInput("test".to_string())),
            "UserInput(\"test\")"
        );
        assert_eq!(
            format!("{:?}", AppEvent::AgentMessage("response".to_string())),
            "AgentMessage(\"response\")"
        );

        // Test that complex variants can be created
        let tool_calls = vec![ToolCall {
            function: ToolFunction {
                name: "test_tool".to_string(),
                arguments: Value::Null,
            },
        }];
        let _tool_request = AppEvent::ToolRequest(tool_calls);

        // Test some other variants
        let _error = AppEvent::Error("test error".to_string());
        let _switch_session = AppEvent::SwitchSession("session1".to_string());
        let _switch_agent = AppEvent::SwitchAgent("qwen".to_string(), "default".to_string());
        let _list_sessions = AppEvent::ListSessions;
        let _session_list =
            AppEvent::SessionList(vec!["session1".to_string(), "session2".to_string()]);
        let _session_switched = AppEvent::SessionSwitched("session1".to_string());

        // Test clone functionality
        let original = AppEvent::UserInput("clone_test".to_string());
        let cloned = original.clone();
        if let AppEvent::UserInput(content) = cloned {
            assert_eq!(content, "clone_test");
        } else {
            panic!("Clone failed or wrong variant");
        }
    }

    #[test]
    fn test_chat_message_struct() {
        // Test ChatMessage::user method
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");
        assert!(user_msg.tool_calls.is_none());

        // Test ChatMessage::assistant method
        let assistant_msg = ChatMessage::assistant("Hi there");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Hi there");
        assert!(assistant_msg.tool_calls.is_none());

        // Test ChatMessage::system method
        let system_msg = ChatMessage::system("System message");
        assert_eq!(system_msg.role, "system");
        assert_eq!(system_msg.content, "System message");
        assert!(system_msg.tool_calls.is_none());

        // Test ChatMessage::tool_call method
        let tool_calls = vec![ToolCall {
            function: ToolFunction {
                name: "test_tool".to_string(),
                arguments: Value::Null,
            },
        }];
        let tool_call_msg = ChatMessage::tool_call("Tool call content", tool_calls.clone());
        assert_eq!(tool_call_msg.role, "assistant"); // Should default to assistant
        assert_eq!(tool_call_msg.content, "Tool call content");
        assert!(tool_call_msg.tool_calls.is_some());

        // Test clone functionality
        let original = ChatMessage::user("clone test");
        let cloned = original.clone();
        assert_eq!(cloned.role, "user");
        assert_eq!(cloned.content, "clone test");
    }

    #[test]
    fn test_tool_call_and_tool_function() {
        // Test ToolCall and ToolFunction creation
        let tool_function = ToolFunction {
            name: "test_function".to_string(),
            arguments: Value::String("test_args".to_string()),
        };

        let tool_call = ToolCall {
            function: tool_function.clone(),
        };

        assert_eq!(tool_call.function.name, "test_function");
        assert_eq!(
            tool_call.function.arguments,
            Value::String("test_args".to_string())
        );

        // Test clone functionality
        let cloned_tool_call = tool_call.clone();
        assert_eq!(cloned_tool_call.function.name, "test_function");
    }

    #[test]
    fn test_tool_and_tool_function_definition() {
        // Test Tool creation using the new method
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string"
                }
            }
        });

        let tool = Tool::new("test_tool", "This is a test tool", params);

        assert_eq!(tool.r#type, "function");
        assert_eq!(tool.function.name, "test_tool");
        assert_eq!(tool.function.description, "This is a test tool");

        // Test truncated_description method
        let long_description = "This is a very long description that exceeds 60 characters and should be truncated with an ellipsis at the end";
        let tool_long_desc = Tool::new("long_desc_tool", long_description, Value::Null);
        let truncated = tool_long_desc.truncated_description();
        assert!(truncated.len() <= 63); // 60 + 3 for "..."
        assert!(truncated.ends_with("..."));

        // Test short description (should not be truncated)
        let short_description = "Short desc";
        let tool_short_desc = Tool::new("short_desc_tool", short_description, Value::Null);
        let not_truncated = tool_short_desc.truncated_description();
        assert_eq!(not_truncated, short_description);

        // Test clone functionality
        let cloned_tool = tool.clone();
        assert_eq!(cloned_tool.function.name, "test_tool");
    }

    #[test]
    fn test_serialization_deserialization() {
        // Test that our types can be serialized and deserialized
        let original_msg = ChatMessage::user("Serialized message");
        let serialized = serde_json::to_string(&original_msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original_msg.role, deserialized.role);
        assert_eq!(original_msg.content, deserialized.content);

        // Test ToolCall serialization/deserialization
        let original_tool_call = ToolCall {
            function: ToolFunction {
                name: "serialize_test".to_string(),
                arguments: serde_json::json!({"arg1": "value1"}),
            },
        };
        let serialized_tc = serde_json::to_string(&original_tool_call).unwrap();
        let deserialized_tc: ToolCall = serde_json::from_str(&serialized_tc).unwrap();
        assert_eq!(
            original_tool_call.function.name,
            deserialized_tc.function.name
        );
    }
}
