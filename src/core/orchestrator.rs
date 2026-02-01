use crate::config::LLMConfig;
use crate::core::multi_agent_manager::{AgentId, MultiAgentManager};
use crate::core::session::SessionManager;
use crate::core::tools::ToolRegistry;
use crate::types::{AppEvent, ChatMessage};
use tokio::sync::mpsc;
use tracing::error;

#[allow(dead_code)] // Some fields kept for future use
pub struct Orchestrator {
    multi_agent_manager: MultiAgentManager,
    active_agent_id: Option<AgentId>,
    session_file: String,
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    model: String,
    llm_config: LLMConfig,
}

impl Orchestrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        system_prompt: &str,
        tool_registry: ToolRegistry,
        session_name: Option<String>,
        _no_stream: bool, // managed by individual agents/client now
        tx: mpsc::Sender<AppEvent>,
        rx: mpsc::Receiver<AppEvent>,
        model: String,
        llm_config: LLMConfig,
    ) -> Self {
        let session_file = match session_name {
            Some(name) => format!("session_{}.json", name),
            None => "session.json".to_string(),
        };

        // Create broadcast channel for MultiAgentManager
        let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(500);

        // Bridge broadcast events to mpsc (for TUI)
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        if let Err(e) = tx_clone.send(event).await {
                            error!("Failed to forward event to TUI: {}", e);
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        error!("Bridge lagged, skipped {} events", skipped);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        let multi_agent_manager = MultiAgentManager::new(
            system_prompt.to_string(),
            tool_registry,
            llm_config.clone(),
            event_tx, // Pass the broadcast sender
        );

        Self {
            multi_agent_manager,
            active_agent_id: None,
            session_file,
            tx,
            rx,
            model,
            llm_config,
        }
    }

    pub fn list_sessions() -> anyhow::Result<Vec<String>> {
        SessionManager::list_sessions()
    }

    pub fn load_state(&mut self) -> anyhow::Result<()> {
        // State loading is now handled per-agent in MultiAgentManager
        // This method might be deprecated or used to restore the *active* agent
        Ok(())
    }

    fn save_state(&mut self) -> anyhow::Result<()> {
        // State saving is handled per-agent in MultiAgentManager
        Ok(())
    }

    #[allow(dead_code)]
    pub fn switch_session(&mut self, _session_name: Option<String>) -> anyhow::Result<()> {
        if let Some(_agent_id) = &self.active_agent_id {
            // Session switching is now async and handled in the run() loop
            return Ok(());
        }
        Ok(())
    }

    pub async fn initialize_default_agent(
        &mut self,
        session_name: Option<String>,
        model: String,
    ) -> anyhow::Result<()> {
        self.model = model.clone();

        // Check if default agent exists
        if let Some(agent) = self.multi_agent_manager.get_agent_by_name("default").await {
            self.active_agent_id = Some(agent.agent_info.id);
        } else {
            // Create default agent
            let agent_id = self
                .multi_agent_manager
                .create_agent("default", &model, session_name)
                .await?;
            self.active_agent_id = Some(agent_id);
        }
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        while let Some(event) = self.rx.recv().await {
            match event {
                AppEvent::UserInput(input) => {
                    if let Some(agent_id) = &self.active_agent_id {
                        if let Err(e) = self
                            .multi_agent_manager
                            .send_event_to_agent(agent_id, AppEvent::UserInput(input))
                            .await
                        {
                            self.tx.send(AppEvent::Error(e.to_string())).await?;
                        }
                    } else {
                        self.tx
                            .send(AppEvent::Error("No active agent".to_string()))
                            .await?;
                    }
                }
                AppEvent::ToolApproval(response) => {
                    if let Some(agent_id) = &self.active_agent_id {
                        if let Err(e) = self
                            .multi_agent_manager
                            .send_event_to_agent(agent_id, AppEvent::ToolApproval(response))
                            .await
                        {
                            self.tx.send(AppEvent::Error(e.to_string())).await?;
                        }
                    }
                }
                AppEvent::SwitchSession(session_name) => {
                    if let Some(agent_id) = &self.active_agent_id {
                        // Forward session switch request directly to the agent task
                        if let Err(e) = self
                            .multi_agent_manager
                            .send_event_to_agent(agent_id, AppEvent::SwitchSession(session_name))
                            .await
                        {
                            self.tx
                                .send(AppEvent::Error(format!("Failed to switch session: {}", e)))
                                .await?;
                        }
                    } else {
                        self.tx
                            .send(AppEvent::Error(
                                "No active agent to switch session for".to_string(),
                            ))
                            .await?;
                    }
                }

                AppEvent::SwitchAgent(agent_name, current_session) => {
                    // Check if agent exists
                    if let Some(agent) = self
                        .multi_agent_manager
                        .get_agent_by_name(&agent_name)
                        .await
                    {
                        let new_agent_id = agent.agent_info.id;
                        self.active_agent_id = Some(new_agent_id.clone());
                        self.tx
                            .send(AppEvent::AgentMessage(format!(
                                "Switched to agent: {}",
                                agent_name
                            )))
                            .await?;

                        // Update Active Model in TUI
                        let model = agent.session_state.read().await.model().to_string();
                        self.tx.send(AppEvent::SwitchModel(model)).await?;

                        // Migrate current session to the new agent
                        if let Err(e) = self
                            .multi_agent_manager
                            .send_event_to_agent(
                                &new_agent_id,
                                AppEvent::SwitchSession(current_session),
                            )
                            .await
                        {
                            self.tx
                                .send(AppEvent::Error(format!("Failed to migrate session: {}", e)))
                                .await?;
                        }
                    } else {
                        // Determine model based on agent name (simple mapping for now)
                        let model = match agent_name.as_str() {
                            "Qwen" => "qwen3:4b",
                            "Llama" => "llama3.2",
                            "Granite" => "granite3.3",
                            _ => "qwen3:4b", // default fallback
                        };

                        // Create new agent
                        match self
                            .multi_agent_manager
                            .create_agent(&agent_name, model, None)
                            .await
                        {
                            Ok(agent_id) => {
                                self.active_agent_id = Some(agent_id.clone());
                                self.tx
                                    .send(AppEvent::AgentMessage(format!(
                                        "Switched to agent: {}",
                                        agent_name
                                    )))
                                    .await?;
                                self.tx
                                    .send(AppEvent::SwitchModel(model.to_string()))
                                    .await?;

                                // Migrate current session to the new agent
                                if let Err(e) = self
                                    .multi_agent_manager
                                    .send_event_to_agent(
                                        &agent_id,
                                        AppEvent::SwitchSession(current_session),
                                    )
                                    .await
                                {
                                    self.tx
                                        .send(AppEvent::Error(format!(
                                            "Failed to migrate session: {}",
                                            e
                                        )))
                                        .await?;
                                }
                            }
                            Err(e) => {
                                self.tx
                                    .send(AppEvent::Error(format!("Failed to create agent: {}", e)))
                                    .await?;
                            }
                        }
                    }
                }
                AppEvent::SwitchModel(model_name) => {
                    // This is harder with MultiAgentManager as model is tied to agent creation usually.
                    // But we can update it in session state.
                    // For now, we might want to just restart the agent or update state?
                    // MultiAgentManager doesn't expose "change model" directly on handle, but session state has it.
                    self.model = model_name.clone();
                    self.tx
                        .send(AppEvent::AgentMessage(format!(
                            "Switched to model: {}",
                            model_name
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
                    if let Some(agent_id) = &self.active_agent_id {
                        if let Err(e) = self
                            .multi_agent_manager
                            .send_event_to_agent(agent_id, AppEvent::ContinueConversation)
                            .await
                        {
                            self.tx.send(AppEvent::Error(e.to_string())).await?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    // Helper methods handle_user_input, handle_tool_approval, chat_with_agent, execute_tool, save_state are removed
    // as their logic is now handled by MultiAgentManager and the run loop.

    pub fn get_session_history(&self) -> Vec<ChatMessage> {
        // Return empty for now as verifying this synchronously is hard with MultiAgentManager
        // TUI should rely on SessionHistory event
        vec![]
    }
}
