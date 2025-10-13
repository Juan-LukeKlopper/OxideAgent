//! Unit tests for the agents module.

use OxideAgent::core::agents::{Agent, AgentId};
use OxideAgent::types::ChatMessage;

#[test]
fn test_agent_new() {
    let agent = Agent::new("TestAgent", "test-model");

    assert_eq!(agent.model, "test-model");
    assert_eq!(agent.history.len(), 1); // System message
    assert_eq!(agent.history[0].role, "system");
    assert!(
        agent.history[0]
            .content
            .contains("You are a helpful assistant")
    );
}

#[test]
fn test_agent_add_user_message() {
    let mut agent = Agent::new("TestAgent", "test-model");

    agent.add_user_message("Hello, world!");

    assert_eq!(agent.history.len(), 2);
    assert_eq!(agent.history[1].role, "user");
    assert_eq!(agent.history[1].content, "Hello, world!");
}

#[test]
fn test_agent_add_assistant_message() {
    let mut agent = Agent::new("TestAgent", "test-model");

    let assistant_message = ChatMessage::assistant("Hello, user!");
    agent.add_assistant_message(assistant_message);

    assert_eq!(agent.history.len(), 2);
    assert_eq!(agent.history[1].role, "assistant");
    assert_eq!(agent.history[1].content, "Hello, user!");
}

#[test]
fn test_agent_id_display() {
    assert_eq!(format!("{}", AgentId::Ollama), "Ollama");
}

#[test]
fn test_agent_id_debug() {
    let agent_id = AgentId::Ollama;
    let debug_str = format!("{:?}", agent_id);
    assert_eq!(debug_str, "Ollama");
}

#[test]
fn test_agent_history_modifications() {
    let mut agent = Agent::new("TestAgent", "test-model");

    // Initially has system message
    assert_eq!(agent.history.len(), 1);

    // Add user message
    agent.add_user_message("First message");
    assert_eq!(agent.history.len(), 2);
    assert_eq!(agent.history[1].content, "First message");

    // Add assistant message
    agent.add_assistant_message(ChatMessage::assistant("Response"));
    assert_eq!(agent.history.len(), 3);
    assert_eq!(agent.history[2].content, "Response");

    // Add another user message
    agent.add_user_message("Second message");
    assert_eq!(agent.history.len(), 4);
    assert_eq!(agent.history[3].content, "Second message");
}

#[test]
fn test_agent_different_models() {
    let agent1 = Agent::new("TestAgent1", "model1");
    let agent2 = Agent::new("TestAgent2", "model2");

    assert_eq!(agent1.model, "model1");
    assert_eq!(agent2.model, "model2");

    // Both should have the same initial system message
    assert_eq!(agent1.history[0].content, agent2.history[0].content);
}
