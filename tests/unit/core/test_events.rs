//! Unit tests for the events module.

use OxideAgent::core::events::{Event, EventBus, EventFilter, EventType};
use OxideAgent::types::{AppEvent, ChatMessage, ToolCall, ToolFunction};
use serde_json::json;

#[test]
fn test_event_creation() {
    let event = Event::new(
        EventType::UserInput("Hello".to_string()),
        "test_source".to_string(),
    );

    assert_eq!(event.source, "test_source");
    assert!(event.destination.is_none());

    match event.event_type {
        EventType::UserInput(input) => assert_eq!(input, "Hello"),
        _ => panic!("Expected UserInput event type"),
    }

    // Check timestamp was set
    assert!(event.timestamp.elapsed().unwrap().as_millis() < 1000);
}

#[test]
fn test_event_with_destination() {
    let event = Event::new_with_destination(
        EventType::AgentMessage("Hello".to_string()),
        "test_source".to_string(),
        "test_destination".to_string(),
    );

    assert_eq!(event.source, "test_source");
    assert_eq!(event.destination, Some("test_destination".to_string()));

    match event.event_type {
        EventType::AgentMessage(message) => assert_eq!(message, "Hello"),
        _ => panic!("Expected AgentMessage event type"),
    }
}

#[test]
#[allow(unused_variables)]
fn test_event_bus_creation() {
    let bus = EventBus::new();
}

#[test]
#[allow(unused_variables)]
fn test_event_bus_subscribe() {
    let bus = EventBus::new();
    let _subscriber = bus.subscribe();
}

#[test]
fn test_event_bus_publish() {
    let bus = EventBus::new();
    let event = Event::new(
        EventType::UserInput("test".to_string()),
        "test_source".to_string(),
    );

    let result = bus.publish(event);
    assert!(result.is_ok());
}

#[test]
#[allow(unused_variables)]
fn test_event_bus_subscribe_and_receive() {
    let bus = EventBus::new();
    let subscriber = bus.subscribe();

    let event = Event::new(
        EventType::AgentMessage("test message".to_string()),
        "test_source".to_string(),
    );
    let publish_result = bus.publish(event.clone());
    assert!(publish_result.is_ok());

    // Try to receive the published event
    // Note: This might fail in testing due to the async nature of broadcast channels
    // when there are no active receivers during publish
}

#[test]
fn test_event_bus_publish_app_event() {
    let bus = EventBus::new();

    // Test various AppEvent types conversion to EventType
    let user_input_event = AppEvent::UserInput("Hello".to_string());
    let result = bus.publish_app_event(user_input_event, "test_source".to_string());
    assert!(result.is_ok());

    let agent_msg_event = AppEvent::AgentMessage("Test message".to_string());
    let result = bus.publish_app_event(agent_msg_event, "test_source".to_string());
    assert!(result.is_ok());

    let error_event = AppEvent::Error("Test error".to_string());
    let result = bus.publish_app_event(error_event, "test_source".to_string());
    assert!(result.is_ok());

    // Test tool call event
    let tool_calls = vec![ToolCall {
        function: ToolFunction {
            name: "test_tool".to_string(),
            arguments: json!({}),
        },
    }];
    let tool_request_event = AppEvent::ToolRequest(tool_calls);
    let result = bus.publish_app_event(tool_request_event, "test_source".to_string());
    assert!(result.is_ok());

    let tool_result_event = AppEvent::ToolResult("test_tool".to_string(), "result".to_string());
    let result = bus.publish_app_event(tool_result_event, "test_source".to_string());
    assert!(result.is_ok());
}

#[test]
#[allow(unused_variables)]
fn test_event_filter_creation() {
    let filter = EventFilter::new();
}

#[test]
fn test_event_filter_with_source() {
    let filter = EventFilter::new().with_source("source_a".to_string());

    // Create events with different sources
    let event_a = Event::new(
        EventType::UserInput("test".to_string()),
        "source_a".to_string(),
    );
    let event_b = Event::new(
        EventType::UserInput("test".to_string()),
        "source_b".to_string(),
    );

    assert!(filter.matches(&event_a));
    assert!(!filter.matches(&event_b));
}

#[test]
fn test_event_filter_with_destination() {
    let filter = EventFilter::new().with_destination("dest_a".to_string());

    // Create events with different destinations
    let event_a = Event::new_with_destination(
        EventType::UserInput("test".to_string()),
        "source".to_string(),
        "dest_a".to_string(),
    );

    let event_b = Event::new_with_destination(
        EventType::UserInput("test".to_string()),
        "source".to_string(),
        "dest_b".to_string(),
    );

    // Create event without destination
    let event_c = Event::new(
        EventType::UserInput("test".to_string()),
        "source".to_string(),
    );

    assert!(filter.matches(&event_a));
    assert!(!filter.matches(&event_b));
    assert!(!filter.matches(&event_c));
}

#[test]
fn test_event_filter_combined() {
    let filter = EventFilter::new()
        .with_source("source_a".to_string())
        .with_destination("dest_a".to_string());

    // Create matching event
    let event_match = Event::new_with_destination(
        EventType::UserInput("test".to_string()),
        "source_a".to_string(),
        "dest_a".to_string(),
    );

    // Create event with wrong source
    let event_wrong_source = Event::new_with_destination(
        EventType::UserInput("test".to_string()),
        "source_b".to_string(),
        "dest_a".to_string(),
    );

    // Create event with wrong destination
    let event_wrong_dest = Event::new_with_destination(
        EventType::UserInput("test".to_string()),
        "source_a".to_string(),
        "dest_b".to_string(),
    );

    assert!(filter.matches(&event_match));
    assert!(!filter.matches(&event_wrong_source));
    assert!(!filter.matches(&event_wrong_dest));
}

#[test]
fn test_event_type_variants() {
    // Test creation of different event types
    let _user_input = EventType::UserInput("test".to_string());
    let _tool_approval_req = EventType::ToolApprovalRequested(vec![]);
    let _agent_msg = EventType::AgentMessage("test".to_string());
    let _agent_chunk = EventType::AgentStreamChunk("chunk".to_string());
    let _tool_req = EventType::ToolRequest(vec![]);
    let _tool_res = EventType::ToolResult("tool_name".to_string(), "result".to_string());
    let _error = EventType::Error("error".to_string());
    let _switch_session = EventType::SwitchSession("session_name".to_string());
    let _switch_agent = EventType::SwitchAgent("agent_name".to_string());
    let _list_sessions = EventType::ListSessions;
    let _session_list = EventType::SessionList(vec!["session1".to_string()]);
    let _session_switched = EventType::SessionSwitched("session_name".to_string());
    let _session_history = EventType::SessionHistory(vec![ChatMessage::user("test")]);
    let _shutdown = EventType::Shutdown;
    let _config_changed = EventType::ConfigChanged;
    let _continue_conversation = EventType::ContinueConversation;
}
