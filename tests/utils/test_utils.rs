//! Test utilities and mock objects for the OxideAgent project.

use OxideAgent::core::agents::Agent;
use OxideAgent::core::interface::{EventEmitter, InputHandler, Interface, OutputHandler};
use OxideAgent::core::tools::{Tool, ToolProfile};
use OxideAgent::types::{AppEvent, ChatMessage};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

/// A mock tool for testing purposes
#[allow(dead_code)]
pub struct MockTool {
    name: String,
    description: String,
    parameters: Value,
    profile: ToolProfile,
    should_fail: bool,
    result: String,
}

impl MockTool {
    #[allow(dead_code)]
    pub fn new(
        name: &str,
        description: &str,
        parameters: Value,
        profile: ToolProfile,
        should_fail: bool,
        result: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
            profile,
            should_fail,
            result: result.to_string(),
        }
    }
}

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    fn profile(&self) -> ToolProfile {
        self.profile
    }

    async fn execute(&self, _args: &Value) -> Result<String> {
        if self.should_fail {
            Err(anyhow::anyhow!("Mock tool failed as requested"))
        } else {
            Ok(self.result.clone())
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(MockTool {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
            profile: self.profile,
            should_fail: self.should_fail,
            result: self.result.clone(),
        })
    }
}

/// A mock interface for testing purposes
#[allow(dead_code)]
pub struct MockInterface {
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    session_name: String,
    history: Vec<ChatMessage>,
}

impl MockInterface {
    #[allow(dead_code)]
    pub fn new(session_name: String) -> (Self, mpsc::Sender<AppEvent>, mpsc::Receiver<AppEvent>) {
        let (tx, rx) = mpsc::channel(32);
        let (interface_tx, interface_rx) = mpsc::channel(32);

        let interface = Self {
            tx: interface_tx,
            rx: interface_rx,
            session_name,
            history: Vec::new(),
        };

        (interface, tx, rx)
    }
}

#[async_trait]
impl InputHandler for MockInterface {
    async fn handle_input(&mut self, input: String) -> Result<()> {
        self.tx.send(AppEvent::UserInput(input)).await?;
        Ok(())
    }
}

#[async_trait]
impl OutputHandler for MockInterface {
    async fn send_output(&mut self, output: AppEvent) -> Result<()> {
        // In a real implementation, we would process the output
        // For testing, we'll just acknowledge it
        match output {
            AppEvent::AgentMessage(content) => {
                self.history.push(ChatMessage::assistant(&content));
            }
            AppEvent::AgentStreamChunk(chunk) => {
                if let Some(last_message) = self.history.last_mut() {
                    let ChatMessage { content, .. } = last_message;
                    {
                        content.push_str(&chunk);
                    }
                } else {
                    self.history.push(ChatMessage::assistant(&chunk));
                }
            }
            AppEvent::ToolResult(name, result) => {
                let content = format!("Tool '{}' result: {}", name, result);
                self.history.push(ChatMessage::assistant(&content));
            }
            _ => {
                // Ignore other events for now
            }
        }
        Ok(())
    }
}

impl EventEmitter for MockInterface {
    fn get_event_sender(&self) -> mpsc::Sender<AppEvent> {
        self.tx.clone()
    }

    fn get_event_receiver(&mut self) -> mpsc::Receiver<AppEvent> {
        std::mem::replace(&mut self.rx, mpsc::channel(1).1)
    }
}

#[async_trait]
impl Interface for MockInterface {
    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        // For testing, we don't need to run anything
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_session_history(&self) -> Vec<ChatMessage> {
        self.history.clone()
    }

    fn get_session_name(&self) -> String {
        self.session_name.clone()
    }
}

/// Create a mock agent for testing
#[allow(dead_code)]
pub fn create_mock_agent() -> Agent {
    let client = Box::new(OxideAgent::core::mocks::MockOllamaClient::new());
    Agent::new("MockAgent", client)
}

/// Create a test configuration
#[allow(dead_code)]
pub fn create_test_config() -> OxideAgent::config::OxideConfig {
    OxideAgent::config::OxideConfig {
        agent: OxideAgent::config::AgentConfig {
            agent_type: OxideAgent::config::AgentType::Qwen,
            model: "qwen:latest".to_string(),
            name: "Qwen".to_string(),
            system_prompt: "You are a test agent.".to_string(),
        },
        no_stream: false,
        session: Some("test_session".to_string()),
        list_sessions: false,
        mcp: OxideAgent::config::MCPConfig {
            server: None,
            auth_token: None,
            tools: vec![],
        },
        interface: OxideAgent::config::InterfaceType::Tui,
        llm: OxideAgent::config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: "http://localhost:11343".to_string(),
            api_key: None,
            model: None,
        },
        multi_agent: OxideAgent::config::MultiAgentConfig::default(),
        web: None,
        telegram: None,
        discord: None,
    }
}
