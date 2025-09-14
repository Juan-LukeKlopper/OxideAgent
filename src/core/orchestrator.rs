use crate::core::agents::Agent;
use crate::core::session::{SessionManager, SessionState};
use crate::core::tools::ToolRegistry;
use crate::types::{AppEvent, ChatMessage, ToolApprovalResponse, ToolCall};
use reqwest::Client;
use tokio::sync::mpsc;

pub struct Orchestrator {
    agent: Agent,
    tool_registry: ToolRegistry,
    client: Client,
    session_file: String,
    no_stream: bool,
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    pending_tool_calls: Option<Vec<ToolCall>>,
}

impl Orchestrator {
    pub fn new(
        agent: Agent,
        tool_registry: ToolRegistry,
        session_name: Option<String>,
        no_stream: bool,
        tx: mpsc::Sender<AppEvent>,
        rx: mpsc::Receiver<AppEvent>,
    ) -> Self {
        let session_file = match session_name {
            Some(name) => format!("session_{}.json", name),
            None => "session.json".to_string(),
        };

        Self {
            agent,
            tool_registry,
            client: Client::new(),
            session_file,
            no_stream,
            tx,
            rx,
            pending_tool_calls: None,
        }
    }

    pub fn list_sessions() -> anyhow::Result<Vec<String>> {
        SessionManager::list_sessions()
    }

    pub fn load_state(&mut self) -> anyhow::Result<()> {
        if let Some(session_state) = SessionManager::load_state(&self.session_file)? {
            self.agent.history = session_state.history().clone();
        }
        Ok(())
    }

    fn save_state(&mut self) -> anyhow::Result<()> {
        let mut session_state = SessionState::new();
        session_state.set_history(self.agent.history.clone());
        SessionManager::save_state(&self.session_file, &session_state)?;
        Ok(())
    }

    pub fn switch_session(&mut self, session_name: Option<String>) -> anyhow::Result<()> {
        // Save current state first
        self.save_state()?;

        // Update session file name
        self.session_file = SessionManager::get_session_filename(session_name.as_deref());

        // Reset agent history
        self.agent.history.clear();

        // Load new session
        self.load_state()?;

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
                AppEvent::ToolApproval(response) => {
                    if let Err(e) = self.handle_tool_approval(response).await {
                        self.tx.send(AppEvent::Error(e.to_string())).await?;
                    }
                }
                AppEvent::SwitchSession(session_name) => {
                    let session_opt = if session_name == "default" {
                        None
                    } else {
                        Some(session_name)
                    };

                    // Clone the session_opt for the display name before moving it
                    let display_name = session_opt.clone().unwrap_or_else(|| "default".to_string());

                    if let Err(e) = self.switch_session(session_opt) {
                        self.tx.send(AppEvent::Error(e.to_string())).await?;
                    } else {
                        // Notify TUI that session has been switched
                        self.tx
                            .send(AppEvent::SessionSwitched(display_name))
                            .await?;

                        // Send the session history to the TUI
                        self.tx
                            .send(AppEvent::SessionHistory(self.get_session_history().clone()))
                            .await?;
                    }
                }
                AppEvent::SwitchAgent(agent_name) => {
                    // Create a new agent with the specified name
                    let agent_type = match agent_name.as_str() {
                        "Qwen" => crate::cli::AgentType::Qwen,
                        "Llama" => crate::cli::AgentType::Llama,
                        "Granite" => crate::cli::AgentType::Granite,
                        _ => {
                            self.tx
                                .send(AppEvent::Error(format!("Unknown agent: {}", agent_name)))
                                .await?;
                            continue;
                        }
                    };

                    // Create new agent
                    let new_agent =
                        crate::core::agents::Agent::new(agent_type.name(), agent_type.model());

                    // Replace the current agent
                    self.agent = new_agent;

                    // Notify TUI that agent has been switched
                    self.tx
                        .send(AppEvent::AgentMessage(format!(
                            "Switched to agent: {}",
                            agent_name
                        )))
                        .await?;
                }
                AppEvent::ListSessions => match Orchestrator::list_sessions() {
                    Ok(sessions) => {
                        let session_list = sessions.join(", ");
                        self.tx
                            .send(AppEvent::AgentMessage(format!(
                                "Available sessions: {}",
                                session_list
                            )))
                            .await?;
                    }
                    Err(e) => {
                        self.tx.send(AppEvent::Error(e.to_string())).await?;
                    }
                },
                AppEvent::RefreshSessions => match Orchestrator::list_sessions() {
                    Ok(sessions) => {
                        self.tx.send(AppEvent::SessionList(sessions)).await?;
                    }
                    Err(e) => {
                        self.tx.send(AppEvent::Error(e.to_string())).await?;
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn handle_user_input(&mut self, input: &str) -> anyhow::Result<()> {
        // If no pending tool calls, this is a new user message
        self.agent.add_user_message(input);
        self.chat_with_agent().await
    }

    async fn handle_tool_approval(&mut self, response: ToolApprovalResponse) -> anyhow::Result<()> {
        if let Some(tool_calls) = self.pending_tool_calls.take() {
            match response {
                ToolApprovalResponse::Allow => {
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
                            tool_call.function.name, tool_output
                        ));
                    }
                    self.save_state()?;
                    // Let the agent process the tool output
                    return self.chat_with_agent().await;
                }
                ToolApprovalResponse::Deny => {
                    self.agent
                        .add_user_message("Tool execution denied by user.");
                    self.tx
                        .send(AppEvent::AgentMessage("Tool execution denied.".to_string()))
                        .await?;
                    self.save_state()?;
                    return Ok(());
                }
                // TODO: Implement AlwaysAllow and AlwaysAllowSession
                _ => {
                    self.tx
                        .send(AppEvent::Error(
                            "This approval mode is not implemented yet.".to_string(),
                        ))
                        .await?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn chat_with_agent(&mut self) -> anyhow::Result<()> {
        // Removed the loop since it only iterates once
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
            } else if self.no_stream {
                // For non-streaming responses, we need to send AgentMessage to display the content
                self.tx
                    .send(AppEvent::AgentMessage(response.content.clone()))
                    .await?;
            }
            // For streaming responses, the content is already displayed via AgentStreamChunk events
            // The streaming case is handled by the UI, which accumulates chunks into a message
        }
        self.save_state()?;
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

    pub fn get_session_history(&self) -> &Vec<ChatMessage> {
        &self.agent.history
    }
}
