use crate::core::agents::Agent;
use crate::core::session::{SessionManager, SessionState};
use crate::core::tool_permissions::GlobalToolPermissions;
use crate::core::tools::ToolRegistry;
use crate::types::{AppEvent, ChatMessage, ToolApprovalResponse, ToolCall};
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::info;

pub struct Orchestrator {
    agent: Agent,
    tool_registry: ToolRegistry,
    client: Client,
    session_file: String,
    session_state: SessionState,
    no_stream: bool,
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    pending_tool_calls: Option<Vec<ToolCall>>,
    global_permissions: GlobalToolPermissions,
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

        let session_state = SessionState::new();
        let global_permissions = GlobalToolPermissions::load().unwrap_or_default();

        Self {
            agent,
            tool_registry,
            client: Client::new(),
            session_file,
            session_state,
            no_stream,
            tx,
            rx,
            pending_tool_calls: None,
            global_permissions,
        }
    }

    pub fn list_sessions() -> anyhow::Result<Vec<String>> {
        SessionManager::list_sessions()
    }

    pub fn load_state(&mut self) -> anyhow::Result<()> {
        match SessionManager::load_state(&self.session_file)? {
            Some(session_state) => {
                self.session_state = session_state;
            }
            None => {
                // File doesn't exist, use default session state
                self.session_state = SessionState::new();
            }
        }
        Ok(())
    }

    fn save_state(&mut self) -> anyhow::Result<()> {
        let mut session_state = self.session_state.clone();
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
        self.session_state = SessionState::new();

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
                    // After handling tool approval, continue the conversation
                    if let Err(e) = self.chat_with_agent().await {
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
                AppEvent::ContinueConversation => {
                    // Continue the conversation after tool execution
                    if let Err(e) = self.chat_with_agent().await {
                        self.tx.send(AppEvent::Error(e.to_string())).await?;
                    }
                }
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
                    // Send event to continue conversation
                    self.tx.send(AppEvent::ContinueConversation).await?;
                }
                ToolApprovalResponse::AlwaysAllow => {
                    // Add tools to global permissions
                    for tool_call in &tool_calls {
                        self.global_permissions
                            .add_allowed(&tool_call.function.name);
                    }
                    // Save global permissions
                    if let Err(e) = self.global_permissions.save() {
                        self.tx
                            .send(AppEvent::Error(format!(
                                "Failed to save global tool permissions: {}",
                                e
                            )))
                            .await?;
                    }

                    // Execute tools
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
                    // Send event to continue conversation
                    self.tx.send(AppEvent::ContinueConversation).await?;
                }
                ToolApprovalResponse::AlwaysAllowSession => {
                    // Add tools to session permissions
                    for tool_call in &tool_calls {
                        self.session_state
                            .add_allowed_tool(tool_call.function.name.clone());
                    }

                    // Execute tools
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
                    // Send event to continue conversation
                    self.tx.send(AppEvent::ContinueConversation).await?;
                }
                ToolApprovalResponse::Deny => {
                    self.agent
                        .add_user_message("Tool execution denied by user.");
                    self.tx
                        .send(AppEvent::AgentMessage("Tool execution denied.".to_string()))
                        .await?;
                    self.save_state()?;
                }
            }
        }
        Ok(())
    }

    async fn chat_with_agent(&mut self) -> anyhow::Result<()> {
        // Removed the loop since it only iterates once
        let tool_definitions = self.tool_registry.definitions();

        // Log the tools being sent for debugging
        info!("=== ORCHESTRATOR CHAT REQUEST START ===");
        info!(
            "Preparing to send chat request with {} tools",
            tool_definitions.len()
        );
        for (i, tool) in tool_definitions.iter().enumerate() {
            info!(
                "  {}. Tool: {} - {}",
                i + 1,
                tool.function.name,
                tool.truncated_description()
            );
        }

        info!("Sending chat request to agent...");
        let response = self
            .agent
            .chat(
                &self.client,
                &tool_definitions,
                !self.no_stream,
                self.tx.clone(),
            )
            .await?;

        info!("=== ORCHESTRATOR CHAT REQUEST END ===");

        if let Some(response) = response {
            if let Some(tool_calls) = &response.tool_calls {
                info!("=== ORCHESTRATOR RECEIVED TOOL CALLS ===");
                info!("Received {} tool calls from agent", tool_calls.len());

                // Check if all tools are already approved
                let all_approved = tool_calls.iter().all(|tool_call| {
                    // Check global permissions first
                    if self.global_permissions.is_allowed(&tool_call.function.name) {
                        info!("Tool '{}' is globally approved", tool_call.function.name);
                        return true;
                    }
                    // Check session permissions
                    if self.session_state.is_tool_allowed(&tool_call.function.name) {
                        info!("Tool '{}' is session approved", tool_call.function.name);
                        return true;
                    }
                    info!("Tool '{}' requires approval", tool_call.function.name);
                    // Not approved
                    false
                });

                if all_approved {
                    info!("All tool calls are approved, executing automatically...");
                    // Execute all tools without approval
                    for (i, tool_call) in tool_calls.iter().enumerate() {
                        info!(
                            "Executing tool call {}: {} with args: {}",
                            i + 1,
                            tool_call.function.name,
                            tool_call.function.arguments
                        );
                        let tool_output = self.execute_tool(tool_call).await?;
                        info!(
                            "Tool '{}' completed with output: {}",
                            tool_call.function.name, tool_output
                        );
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
                } else {
                    info!("Some tool calls require approval, requesting user approval...");
                    // Send tool calls for approval
                    self.tx
                        .send(AppEvent::ToolRequest(tool_calls.clone()))
                        .await?;
                    self.pending_tool_calls = Some(tool_calls.clone());
                }
                info!("=== ORCHESTRATOR TOOL CALL PROCESSING END ===");
            } else if self.no_stream {
                // For non-streaming responses, we need to send AgentMessage to display the content
                info!(
                    "Non-streaming response received, sending to UI: {} chars",
                    response.content.len()
                );
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
            tool.execute(&tool_call.function.arguments).await
        } else {
            let error_msg = format!("Unknown tool: {}", tool_call.function.name);
            self.tx.send(AppEvent::Error(error_msg.clone())).await?;
            Err(anyhow::anyhow!(error_msg))
        }
    }

    pub fn get_session_history(&self) -> &Vec<ChatMessage> {
        self.session_state.history()
    }
}
