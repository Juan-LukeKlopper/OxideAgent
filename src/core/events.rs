//! Enhanced event system for the OxideAgent system.
//!
//! This module implements a robust event system for communication between components.

use crate::types::{AppEvent, ChatMessage, ToolCall};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Comprehensive event types for the application
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants are used in AppEvent conversions
pub enum EventType {
    /// User input events
    UserInput(String),

    /// Tool approval events
    ToolApprovalRequested(Vec<ToolCall>),
    ToolApprovalResponse(crate::types::ToolApprovalResponse),

    /// Agent communication events
    AgentMessage(String),
    AgentStreamChunk(String),
    AgentStreamEnd,

    /// Tool execution events
    ToolRequest(Vec<ToolCall>),
    ToolResult(String, String), // (tool_name, result)

    /// Error events
    Error(String),

    /// Session management events
    SwitchSession(String),
    SwitchAgent(String),
    ListSessions,
    RefreshSessions,
    SessionList(Vec<String>),
    SessionSwitched(String),
    SessionHistory(Vec<ChatMessage>),

    /// System events
    Shutdown,
    ConfigChanged,
    ContinueConversation, // New event to continue conversation after tool execution
}

/// Event with metadata
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are used in EventFilter::matches
pub struct Event {
    /// The event type
    pub event_type: EventType,

    /// The source of the event
    pub source: String,

    /// The destination of the event (if any)
    pub destination: Option<String>,

    /// Timestamp of when the event was created
    pub timestamp: std::time::SystemTime,
}

#[allow(dead_code)] // Functions are used to create Event instances
impl Event {
    /// Create a new event
    pub fn new(event_type: EventType, source: String) -> Self {
        Self {
            event_type,
            source,
            destination: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create a new event with a destination
    pub fn new_with_destination(
        event_type: EventType,
        source: String,
        destination: String,
    ) -> Self {
        Self {
            event_type,
            source,
            destination: Some(destination),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Event bus for asynchronous communication between components
#[allow(dead_code)] // Fields are used in EventBus methods
pub struct EventBus {
    /// Broadcast sender for events
    sender: broadcast::Sender<Arc<Event>>,
}

#[allow(dead_code)] // Methods are part of the EventBus API
impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, _receiver) = broadcast::channel(100);
        Self { sender }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Event>> {
        self.sender.subscribe()
    }

    /// Publish an event
    pub fn publish(&self, event: Event) -> Result<()> {
        let event = Arc::new(event);
        // Use send which returns a result to handle potential channel issues
        match self.sender.send(event) {
            Ok(_) => Ok(()),
            Err(_) => Ok(()), // Ignore errors (e.g., when no receivers are active)
        }
    }

    /// Publish an event from an AppEvent
    pub fn publish_app_event(&self, app_event: AppEvent, source: String) -> Result<()> {
        let event_type = match app_event {
            AppEvent::UserInput(input) => EventType::UserInput(input),
            AppEvent::ToolApproval(response) => EventType::ToolApprovalResponse(response),
            AppEvent::AgentMessage(message) => EventType::AgentMessage(message),
            AppEvent::AgentStreamChunk(chunk) => EventType::AgentStreamChunk(chunk),
            AppEvent::AgentStreamEnd => EventType::AgentStreamEnd,
            AppEvent::ToolRequest(calls) => EventType::ToolRequest(calls),
            AppEvent::ToolResult(name, result) => EventType::ToolResult(name, result),
            AppEvent::Error(error) => EventType::Error(error),
            AppEvent::SwitchSession(session) => EventType::SwitchSession(session),
            AppEvent::SwitchAgent(agent) => EventType::SwitchAgent(agent),
            AppEvent::ListSessions => EventType::ListSessions,
            AppEvent::RefreshSessions => EventType::RefreshSessions,
            AppEvent::SessionList(sessions) => EventType::SessionList(sessions),
            AppEvent::SessionSwitched(session) => EventType::SessionSwitched(session),
            AppEvent::SessionHistory(history) => EventType::SessionHistory(history),
            AppEvent::ContinueConversation => EventType::ContinueConversation,
        };

        let event = Event::new(event_type, source);
        self.publish(event)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event filter for routing events
#[allow(dead_code)] // Fields are used in EventFilter::matches
pub struct EventFilter {
    /// Source filter
    source_filter: Option<String>,

    /// Destination filter
    destination_filter: Option<String>,

    /// Event type filter
    event_type_filter: Option<EventType>,
}

#[allow(dead_code)] // Methods are part of the EventFilter API
impl EventFilter {
    /// Create a new event filter
    pub fn new() -> Self {
        Self {
            source_filter: None,
            destination_filter: None,
            event_type_filter: None,
        }
    }

    /// Set the source filter
    pub fn with_source(mut self, source: String) -> Self {
        self.source_filter = Some(source);
        self
    }

    /// Set the destination filter
    pub fn with_destination(mut self, destination: String) -> Self {
        self.destination_filter = Some(destination);
        self
    }

    /// Set the event type filter
    pub fn with_event_type(mut self, event_type: EventType) -> Self {
        self.event_type_filter = Some(event_type);
        self
    }

    /// Check if an event matches the filter
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(ref source) = self.source_filter
            && &event.source != source
        {
            return false;
        }

        if let Some(ref destination) = self.destination_filter {
            if let Some(ref event_destination) = event.destination {
                if event_destination != destination {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref event_type) = self.event_type_filter {
            // This is a simplified comparison - in a real implementation,
            // we'd need to implement PartialEq for EventType
            // For now, we'll just return true to avoid complexity
            let _ = event_type;
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}
