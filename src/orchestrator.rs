use crate::agents::Agent;
use crate::tools::ToolRegistry;
use crate::types::{AppEvent, ChatMessage, ToolCall};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;

const SESSION_FILE: &str = "session.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionState {
    history: Vec<ChatMessage>,
}

impl SessionState {
    fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }
}

pub struct Orchestrator {
    agent: Agent,
    tool_registry: ToolRegistry,
    client: Client,
    session_state: SessionState,
    no_stream: bool,
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    pending_tool_calls: Option<Vec<ToolCall>>,
}

impl Orchestrator {
    pub fn new(
        agent: Agent,
        tool_registry: ToolRegistry,
        no_stream: bool,
        tx: mpsc::Sender<AppEvent>,
        rx: mpsc::Receiver<AppEvent>,
    ) -> Self {
        Self {
            agent,
            tool_registry,
            client: Client::new(),
            session_state: SessionState::new(),
            no_stream,
            tx,
            rx,
            pending_tool_calls: None,
        }
    }

    pub fn load_state(&mut self) -> anyhow::Result<()> {
        if Path::new(SESSION_FILE).exists() {
            let session_json = fs::read_to_string(SESSION_FILE)?;
            if !session_json.trim().is_empty() {
                let session_state: SessionState = serde_json::from_str(&session_json)?;
                self.agent.history = session_state.history.clone();
                self.session_state = session_state;
            }
        }
        Ok(())
    }

    fn save_state(&mut self) -> anyhow::Result<()> {
        self.session_state.history = self.agent.history.clone();
        let session_json = serde_json::to_string_pretty(&self.session_state)?;
        fs::write(SESSION_FILE, session_json)?;
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        while let Some(event) = self.rx.recv().await {
            match event {
                AppEvent::UserInput(input) => {
                    if let Err(e) = self.handle_user_input(&input).await {
                        self.tx.send(AppEvent::Error(e.to_string())).await?;
                    }
                }
                _ => {} // Ignore other events for now
            }
        }
        Ok(())
    }

    pub async fn handle_user_input(&mut self, input: &str) -> anyhow::Result<()> {
        // If there are pending tool calls, the user input is the approval
        if let Some(tool_calls) = self.pending_tool_calls.take() {
            if input.trim().eq_ignore_ascii_case("y") {
                for tool_call in tool_calls {
                    let tool_output = self.execute_tool(&tool_call).await?;
                    self.tx
                        .send(AppEvent::ToolResult(
                            tool_call.function.name.clone(),
                            tool_output.clone(),
                        ))
                        .await?;
                    self.agent.add_user_message(&format!(
                        "The tool '{}' produced this output:\n{}",
                        tool_call.function.name,
                        tool_output
                    ));
                }
                self.save_state()?;
                // Let the agent process the tool output
                return self.chat_with_agent().await;
            } else {
                self.agent
                    .add_user_message("Tool execution denied by user.");
                self.tx
                    .send(AppEvent::AgentMessage(
                        "Tool execution denied.".to_string(),
                    ))
                    .await?;
                self.save_state()?;
                return Ok(());
            }
        }

        // If no pending tool calls, this is a new user message
        self.agent.add_user_message(input);
        self.chat_with_agent().await
    }

    async fn chat_with_agent(&mut self) -> anyhow::Result<()> {
        'agent_turn: loop {
            let tool_definitions = self.tool_registry.definitions();
            let response = self
                .agent
                .chat(
                    &self.client,
                    &tool_definitions,
                    !self.no_stream,
                    self.tx.clone(),
                )
                .await?;

            if let Some(response) = response {
                if let Some(tool_calls) = &response.tool_calls {
                    self.tx
                        .send(AppEvent::ToolRequest(tool_calls.clone()))
                        .await?;
                    self.pending_tool_calls = Some(tool_calls.clone());
                } else {
                    self.tx
                        .send(AppEvent::AgentMessage(response.content.clone()))
                        .await?;
                }
            }
            self.save_state()?;
            break 'agent_turn;
        }
        Ok(())
    }

    async fn execute_tool(&self, tool_call: &ToolCall) -> anyhow::Result<String> {
        if let Some(tool) = self.tool_registry.get_tool(&tool_call.function.name) {
            tool.execute(&tool_call.function.arguments)
        } else {
            let error_msg = format!("Unknown tool: {}", tool_call.function.name);
            self.tx.send(AppEvent::Error(error_msg.clone())).await?;
            Err(anyhow::anyhow!(error_msg))
        }
    }
}