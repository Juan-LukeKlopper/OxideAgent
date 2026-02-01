use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::config::LLMConfig;
use crate::core::agents::Agent;
use crate::core::session::{SessionManager, SessionState};
use crate::core::tool_permissions::GlobalToolPermissions;
use crate::core::tools::ToolRegistry;
use crate::types::{AppEvent, ToolApprovalResponse, ToolCall};
use tracing::{error, info};

struct ChatContext<'a> {
    agent: &'a mut Agent,
    model: &'a str,
    tool_registry: &'a ToolRegistry,
    sender: mpsc::Sender<AppEvent>,
    event_tx: &'a broadcast::Sender<AppEvent>,
    llm_config: &'a LLMConfig,
    session_state: &'a Arc<RwLock<SessionState>>,
    global_permissions: &'a mut GlobalToolPermissions,
}

struct ApprovalContext<'a> {
    tx: &'a mpsc::Sender<AppEvent>,
    event_tx: &'a broadcast::Sender<AppEvent>,
    agent: &'a mut Agent,
    tool_registry: &'a ToolRegistry,
    session_state: &'a Arc<RwLock<SessionState>>,
    global_permissions: &'a mut GlobalToolPermissions,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct AgentId(String);

impl AgentId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub model: String,
    pub status: AgentStatus,
    pub session_name: String,
}

#[derive(Debug, Clone)]
pub enum AgentStatus {
    Idle,
    Processing,
    Error(String),
}

pub struct AgentHandle {
    pub agent_info: AgentInfo,
    pub task_handle: JoinHandle<anyhow::Result<()>>,
    pub tx: mpsc::Sender<AppEvent>,
    pub session_state: Arc<RwLock<SessionState>>,
}

#[derive(Clone)]
pub struct AgentHandleRef {
    pub agent_info: AgentInfo,
    pub tx: mpsc::Sender<AppEvent>,
    pub session_state: Arc<RwLock<SessionState>>,
}

pub struct MultiAgentManager {
    agents: Arc<RwLock<HashMap<AgentId, AgentHandle>>>,
    tool_registry: ToolRegistry,
    system_prompt: String,
    llm_config: LLMConfig,
    event_tx: broadcast::Sender<AppEvent>,
}

impl MultiAgentManager {
    pub fn new(
        system_prompt: String,
        tool_registry: ToolRegistry,
        llm_config: LLMConfig,
        event_tx: broadcast::Sender<AppEvent>,
    ) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            tool_registry,
            system_prompt,
            llm_config,
            event_tx,
        }
    }

    pub async fn create_agent(
        &self,
        agent_name: &str,
        model: &str,
        session_name: Option<String>,
    ) -> anyhow::Result<AgentId> {
        let agent_id = AgentId::new(&format!("agent_{}", nanoid::nanoid!(8)));

        // Create session filename for the agent
        let session_file = SessionManager::get_session_filename(session_name.as_deref());

        // Load session state if it exists
        let session_state = (SessionManager::load_state(&session_file)?).unwrap_or_default();

        // Create channels for the agent
        let (agent_tx, mut agent_rx) = mpsc::channel(100);

        // Clone necessary references for the task
        let agent_clone = self.system_prompt.clone();
        let tool_registry_clone = self.tool_registry.clone_registry();
        let session_state_clone = Arc::new(RwLock::new(session_state.clone()));
        let session_state_for_task = session_state_clone.clone();
        let llm_config_clone = self.llm_config.clone();
        let event_tx_clone = self.event_tx.clone();
        let name_clone = agent_name.to_string();
        let model_clone = model.to_string();
        let session_name_clone = session_name.unwrap_or_else(|| "default".to_string());

        // Pre-clone values that will be used outside the async task
        let task_agent_name = name_clone.clone();
        let task_agent_model = model_clone.clone();
        let task_agent_tx = agent_tx.clone();
        let task_agent_id_for_task = agent_id.clone();
        let task_agent_id_for_handle = agent_id.clone();
        let task_session_name = session_name_clone.clone();

        // Start the agent task
        let task_handle = tokio::spawn(async move {
            // Create LLM client for this agent
            let llm_client = crate::core::llm::llm_client_factory(&llm_config_clone)
                .expect("Failed to create LLM client");

            let mut agent = Agent::new(&agent_clone, llm_client);
            // Set the model from session or use provided model
            let agent_model = if session_state.model() != "qwen3:4b" {
                session_state.model().to_string()
            } else {
                model_clone.clone()
            };

            // Set agent history from session
            agent.history = session_state.history().clone();

            // Track pending tool calls for this agent
            let mut pending_tool_calls: Option<Vec<ToolCall>> = None;

            // Global permissions for this agent task
            let mut global_permissions = GlobalToolPermissions::load().unwrap_or_default();

            // Track if agent is currently processing to prevent session switch mid-operation
            let mut is_processing = false;

            // Queue for deferred session switch
            let mut pending_session_switch: Option<String> = None;

            // Notify that the agent is starting
            let _ = event_tx_clone.send(AppEvent::AgentStatusUpdate(
                format!("{}-{}", name_clone, task_agent_id_for_task),
                "Active".to_string(),
            ));

            // Create a specialized channel for streaming LLM output to avoid blocking the main loop
            let (stream_tx, mut stream_rx) = mpsc::channel::<AppEvent>(500);

            // Spawn a task to forward stream events directly to the broadcast channel
            let event_tx_stream = event_tx_clone.clone();
            tokio::spawn(async move {
                while let Some(event) = stream_rx.recv().await {
                    let _ = event_tx_stream.send(event);
                }
            });

            let mut current_session_name = task_session_name;

            loop {
                match tokio::time::timeout(std::time::Duration::from_secs(30), agent_rx.recv())
                    .await
                {
                    Ok(Some(event)) => {
                        match event {
                            AppEvent::SwitchSession(new_session_name) => {
                                if is_processing {
                                    // Agent is busy, queue the switch for after processing completes
                                    info!(
                                        "Agent busy, queuing session switch to: {:?}",
                                        new_session_name
                                    );
                                    pending_session_switch = Some(new_session_name);
                                    event_tx_clone.send(AppEvent::AgentMessage(
                                        "Session switch queued - will complete after current response.".to_string()
                                    )).ok();
                                    continue;
                                }

                                info!("Agent switching session to: {:?}", new_session_name);

                                // Save current state
                                {
                                    let state_guard = session_state_for_task.read().await;
                                    let session_file = SessionManager::get_session_filename(
                                        if current_session_name == "default" {
                                            None
                                        } else {
                                            Some(&current_session_name)
                                        },
                                    );
                                    if let Err(e) =
                                        SessionManager::save_state(&session_file, &state_guard)
                                    {
                                        error!("Failed to save session state: {}", e);
                                        event_tx_clone
                                            .send(AppEvent::Error(format!(
                                                "Failed to save session: {}",
                                                e
                                            )))
                                            .ok();
                                        continue;
                                    }
                                }

                                // Load new state
                                let new_name_str = if new_session_name == "default" {
                                    "default"
                                } else {
                                    &new_session_name
                                };
                                let session_opt = if new_session_name == "default" {
                                    None
                                } else {
                                    Some(new_session_name.as_str())
                                };
                                let new_session_file =
                                    SessionManager::get_session_filename(session_opt);

                                match SessionManager::load_state(&new_session_file) {
                                    Ok(loaded_state) => {
                                        let new_state =
                                            loaded_state.unwrap_or_else(SessionState::new);

                                        // Update in-memory state
                                        {
                                            let mut state_guard =
                                                session_state_for_task.write().await;
                                            *state_guard = new_state.clone();
                                        }

                                        // Update agent history
                                        agent.history = new_state.history().clone();

                                        // Update local session name
                                        current_session_name = new_name_str.to_string();

                                        // Notify TUI
                                        event_tx_clone
                                            .send(AppEvent::SessionSwitched(
                                                current_session_name.clone(),
                                            ))
                                            .ok();
                                        event_tx_clone
                                            .send(AppEvent::SessionHistory(agent.history.clone()))
                                            .ok();
                                    }
                                    Err(e) => {
                                        error!("Failed to load session: {}", e);
                                        event_tx_clone
                                            .send(AppEvent::Error(format!(
                                                "Failed to load session: {}",
                                                e
                                            )))
                                            .ok();
                                    }
                                }
                            }
                            AppEvent::UserInput(input) => {
                                // Mark as processing to prevent session switch mid-operation
                                is_processing = true;

                                // Update agent status
                                let _ = event_tx_clone.send(AppEvent::AgentStatusUpdate(
                                    format!("{}-{}", name_clone, task_agent_id_for_task),
                                    "Processing".to_string(),
                                ));

                                // Add user message to agent history
                                agent.add_user_message(&input);

                                // Update session state to reflect new message
                                {
                                    let mut state = session_state_for_task.write().await;
                                    state.set_history(agent.history.clone());
                                }

                                // Send chat request to agent
                                let chat_context = ChatContext {
                                    agent: &mut agent,
                                    model: &agent_model,
                                    tool_registry: &tool_registry_clone,
                                    sender: stream_tx.clone(),
                                    event_tx: &event_tx_clone,
                                    llm_config: &llm_config_clone,
                                    session_state: &session_state_for_task,
                                    global_permissions: &mut global_permissions,
                                };
                                if let Err(e) =
                                    Self::chat_with_agent(chat_context, &mut pending_tool_calls)
                                        .await
                                {
                                    event_tx_clone.send(AppEvent::Error(e.to_string())).ok();
                                }

                                // Mark as done processing
                                is_processing = false;

                                // Update status back to Idle
                                let _ = event_tx_clone.send(AppEvent::AgentStatusUpdate(
                                    format!("{}-{}", name_clone, task_agent_id_for_task),
                                    "Idle".to_string(),
                                ));

                                // Process any pending session switch
                                if let Some(queued_session) = pending_session_switch.take() {
                                    // Re-queue the switch event so it gets processed next iteration
                                    let _ = agent_tx
                                        .send(AppEvent::SwitchSession(queued_session))
                                        .await;
                                }
                            }
                            AppEvent::ToolApproval(response) => {
                                if let Some(tool_calls) = pending_tool_calls.take() {
                                    let approval_context = ApprovalContext {
                                        tx: &agent_tx,
                                        event_tx: &event_tx_clone,
                                        agent: &mut agent,
                                        tool_registry: &tool_registry_clone,
                                        session_state: &session_state_for_task,
                                        global_permissions: &mut global_permissions,
                                    };
                                    if let Err(e) = Self::handle_tool_approval(
                                        approval_context,
                                        &tool_calls,
                                        response,
                                    )
                                    .await
                                    {
                                        event_tx_clone.send(AppEvent::Error(e.to_string())).ok();
                                    }
                                }
                            }
                            AppEvent::ContinueConversation => {
                                // Mark as processing
                                is_processing = true;

                                // Update agent status
                                let _ = event_tx_clone.send(AppEvent::AgentStatusUpdate(
                                    format!("{}-{}", name_clone, task_agent_id_for_task),
                                    "Processing".to_string(),
                                ));

                                // Continue the conversation after tool execution
                                let chat_context = ChatContext {
                                    agent: &mut agent,
                                    model: &agent_model,
                                    tool_registry: &tool_registry_clone,
                                    sender: stream_tx.clone(),
                                    event_tx: &event_tx_clone,
                                    llm_config: &llm_config_clone,
                                    session_state: &session_state_for_task,
                                    global_permissions: &mut global_permissions,
                                };
                                if let Err(e) =
                                    Self::chat_with_agent(chat_context, &mut pending_tool_calls)
                                        .await
                                {
                                    event_tx_clone.send(AppEvent::Error(e.to_string())).ok();
                                }

                                // Mark as done processing
                                is_processing = false;

                                // Update status back to Idle
                                let _ = event_tx_clone.send(AppEvent::AgentStatusUpdate(
                                    format!("{}-{}", name_clone, task_agent_id_for_task),
                                    "Idle".to_string(),
                                ));

                                // Process any pending session switch
                                if let Some(queued_session) = pending_session_switch.take() {
                                    let _ = agent_tx
                                        .send(AppEvent::SwitchSession(queued_session))
                                        .await;
                                }
                            }
                            AppEvent::AgentStatusUpdate(_, _) => {
                                // Ignore status updates sent to agent - these are for TUI
                            }
                            AppEvent::AgentStreamChunk(chunk) => {
                                // Forward streaming chunks to TUI
                                let _ = event_tx_clone.send(AppEvent::AgentStreamChunk(chunk));
                            }
                            AppEvent::AgentStreamEnd => {
                                // Forward stream end to TUI
                                let _ = event_tx_clone.send(AppEvent::AgentStreamEnd);
                            }
                            _ => {
                                // Handle other events if needed
                            }
                        }
                    }
                    Ok(None) => {
                        // Channel closed, exit the loop
                        break;
                    }
                    Err(_) => {
                        // Timeout occurred, update status and continue
                        let _ = event_tx_clone.send(AppEvent::AgentStatusUpdate(
                            format!("{}-{}", name_clone, task_agent_id_for_task),
                            "Idle".to_string(),
                        ));
                    }
                }
            }

            // Notify that the agent task is ending
            let _ = event_tx_clone.send(AppEvent::AgentStatusUpdate(
                format!("{}-{}", name_clone, task_agent_id_for_task),
                "Stopped".to_string(),
            ));

            Ok(())
        });

        // Create agent handle
        let handle = AgentHandle {
            agent_info: AgentInfo {
                id: task_agent_id_for_handle,
                name: task_agent_name,
                model: task_agent_model,
                status: AgentStatus::Idle,
                session_name: session_name_clone,
            },
            task_handle,
            tx: task_agent_tx, // Use the pre-cloned sender
            session_state: session_state_clone,
        };

        // Store the agent handle
        self.agents.write().await.insert(agent_id.clone(), handle);

        // Notify that a new agent was created
        self.event_tx.send(AppEvent::AgentMessage(format!(
            "Created new agent: {} ({})",
            agent_id, agent_name
        )))?;

        Ok(agent_id)
    }

    async fn chat_with_agent(
        context: ChatContext<'_>,
        pending_tool_calls: &mut Option<Vec<ToolCall>>,
    ) -> anyhow::Result<()> {
        let tool_definitions = context.tool_registry.definitions();

        info!("=== MULTI-AGENT CHAT REQUEST START ===");
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

        info!(
            "Sending chat request to agent with model: {}...",
            context.model
        );
        let response = match context
            .agent
            .chat(
                context.model,
                &tool_definitions,
                true, // Enable streaming by default
                context.sender.clone(),
            )
            .await
        {
            Ok(response) => response,
            Err(e) => {
                let error_msg = format!("Error communicating with LLM: {}", e);
                context
                    .event_tx
                    .send(AppEvent::Error(error_msg.clone()))
                    .ok();
                context.agent.add_user_message(&format!("Error: {}", e));
                return Err(e);
            }
        };

        info!("=== MULTI-AGENT CHAT REQUEST END ===");

        if let Some(response) = response
            && let Some(tool_calls) = &response.tool_calls
        {
            info!("=== MULTI-AGENT RECEIVED TOOL CALLS ===");
            info!("Received {} tool calls from agent", tool_calls.len());

            // Read session state once before checking permissions (async-safe)
            let session_state_guard = context.session_state.read().await;

            // Check if all tools are already approved
            let all_approved = tool_calls.iter().all(|tool_call| {
                // Check global permissions first
                if context
                    .global_permissions
                    .is_allowed(&tool_call.function.name)
                {
                    info!("Tool '{}' is globally approved", tool_call.function.name);
                    return true;
                }
                // Check session permissions (using pre-acquired read guard)
                if session_state_guard.is_tool_allowed(&tool_call.function.name) {
                    info!("Tool '{}' is session approved", tool_call.function.name);
                    return true;
                }
                info!("Tool '{}' requires approval", tool_call.function.name);
                // Not approved
                false
            });

            // Drop the read guard before potential writes
            drop(session_state_guard);

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
                    if let Some(tool) = context.tool_registry.get_tool(&tool_call.function.name) {
                        match tool.execute(&tool_call.function.arguments).await {
                            Ok(tool_output) => {
                                info!(
                                    "Tool '{}' completed with output: {}",
                                    tool_call.function.name, tool_output
                                );
                                context
                                    .event_tx
                                    .send(AppEvent::ToolResult(
                                        tool_call.function.name.clone(),
                                        tool_output.clone(),
                                    ))
                                    .ok(); // Use ok() to handle potential broadcast errors gracefully
                                context.agent.add_user_message(&format!(
                                    "The tool '{}' produced this output:\n{}",
                                    tool_call.function.name, tool_output
                                ));
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "Error executing tool '{}': {}",
                                    tool_call.function.name, e
                                );
                                context
                                    .event_tx
                                    .send(AppEvent::Error(error_msg.clone()))
                                    .ok(); // Use ok() to handle potential broadcast errors gracefully
                                context.agent.add_user_message(&format!(
                                    "Error executing tool '{}': {}",
                                    tool_call.function.name, e
                                ));
                            }
                        }
                    } else {
                        let error_msg = format!("Unknown tool: {}", tool_call.function.name);
                        context
                            .event_tx
                            .send(AppEvent::Error(error_msg.clone()))
                            .ok(); // Use ok() to handle potential broadcast errors gracefully
                        context.agent.add_user_message(&error_msg);
                    }
                }
            } else {
                info!("Some tool calls require approval, requesting user approval...");
                // Send tool calls for approval
                context
                    .event_tx
                    .send(AppEvent::ToolRequest(tool_calls.clone()))
                    .ok(); // Use ok() to handle potential broadcast errors gracefully
                *pending_tool_calls = Some(tool_calls.clone());
            }
            info!("=== MULTI-AGENT TOOL CALL PROCESSING END ===");
        }

        // Save the session state after each interaction
        {
            let mut state = context.session_state.write().await;
            state.set_history(context.agent.history.clone());
            state.set_model(context.model.to_string());
        }

        Ok(())
    }

    async fn handle_tool_approval(
        context: ApprovalContext<'_>,
        tool_calls: &[ToolCall],
        response: ToolApprovalResponse,
    ) -> anyhow::Result<()> {
        let tx = context.tx;
        let event_tx = context.event_tx;
        let agent = context.agent;
        let tool_registry = context.tool_registry;
        let session_state = context.session_state;
        let global_permissions = context.global_permissions;

        match response {
            ToolApprovalResponse::Allow => {
                for tool_call in tool_calls {
                    if let Some(tool) = tool_registry.get_tool(&tool_call.function.name) {
                        match tool.execute(&tool_call.function.arguments).await {
                            Ok(tool_output) => {
                                event_tx.send(AppEvent::ToolResult(
                                    tool_call.function.name.clone(),
                                    tool_output.clone(),
                                ))?;
                                agent.add_user_message(&format!(
                                    "The tool '{}' produced this output:\n{}",
                                    tool_call.function.name, tool_output
                                ));
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "Error executing tool '{}': {}",
                                    tool_call.function.name, e
                                );
                                event_tx.send(AppEvent::Error(error_msg.clone()))?;
                                agent.add_user_message(&error_msg);
                            }
                        }
                    } else {
                        let error_msg = format!("Unknown tool: {}", tool_call.function.name);
                        event_tx.send(AppEvent::Error(error_msg.clone()))?;
                        agent.add_user_message(&error_msg);
                    }
                }
                // Send event to continue conversation
                tx.send(AppEvent::ContinueConversation).await?;
            }
            ToolApprovalResponse::AlwaysAllow => {
                // Add tools to global permissions
                for tool_call in tool_calls {
                    global_permissions.add_allowed(&tool_call.function.name);
                }
                // Save global permissions
                if let Err(e) = global_permissions.save() {
                    event_tx.send(AppEvent::Error(format!(
                        "Failed to save global tool permissions: {}",
                        e
                    )))?;
                }

                // Execute tools
                for tool_call in tool_calls {
                    if let Some(tool) = tool_registry.get_tool(&tool_call.function.name) {
                        match tool.execute(&tool_call.function.arguments).await {
                            Ok(tool_output) => {
                                event_tx.send(AppEvent::ToolResult(
                                    tool_call.function.name.clone(),
                                    tool_output.clone(),
                                ))?;
                                agent.add_user_message(&format!(
                                    "The tool '{}' produced this output:\n{}",
                                    tool_call.function.name, tool_output
                                ));
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "Error executing tool '{}': {}",
                                    tool_call.function.name, e
                                );
                                event_tx.send(AppEvent::Error(error_msg.clone()))?;
                                agent.add_user_message(&error_msg);
                            }
                        }
                    } else {
                        let error_msg = format!("Unknown tool: {}", tool_call.function.name);
                        event_tx.send(AppEvent::Error(error_msg.clone()))?;
                        agent.add_user_message(&error_msg);
                    }
                }
                // Send event to continue conversation
                tx.send(AppEvent::ContinueConversation).await?;
            }
            ToolApprovalResponse::AlwaysAllowSession => {
                // Add tools to session permissions
                for tool_call in tool_calls {
                    session_state
                        .write()
                        .await
                        .add_allowed_tool(tool_call.function.name.clone());
                }

                // Execute tools
                for tool_call in tool_calls {
                    if let Some(tool) = tool_registry.get_tool(&tool_call.function.name) {
                        match tool.execute(&tool_call.function.arguments).await {
                            Ok(tool_output) => {
                                event_tx.send(AppEvent::ToolResult(
                                    tool_call.function.name.clone(),
                                    tool_output.clone(),
                                ))?;
                                agent.add_user_message(&format!(
                                    "The tool '{}' produced this output:\n{}",
                                    tool_call.function.name, tool_output
                                ));
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "Error executing tool '{}': {}",
                                    tool_call.function.name, e
                                );
                                event_tx.send(AppEvent::Error(error_msg.clone()))?;
                                agent.add_user_message(&error_msg);
                            }
                        }
                    } else {
                        let error_msg = format!("Unknown tool: {}", tool_call.function.name);
                        event_tx.send(AppEvent::Error(error_msg.clone()))?;
                        agent.add_user_message(&error_msg);
                    }
                }
                // Send event to continue conversation
                tx.send(AppEvent::ContinueConversation).await?;
            }
            ToolApprovalResponse::Deny => {
                agent.add_user_message("Tool execution denied by user.");
                event_tx.send(AppEvent::AgentMessage("Tool execution denied.".to_string()))?;
            }
        }
        Ok(())
    }

    pub async fn get_agent(&self, agent_id: &AgentId) -> Option<AgentHandleRef> {
        self.agents
            .read()
            .await
            .get(agent_id)
            .map(|handle| AgentHandleRef {
                agent_info: handle.agent_info.clone(),
                tx: handle.tx.clone(),
                session_state: handle.session_state.clone(),
            })
    }

    pub async fn get_agent_by_name(&self, name: &str) -> Option<AgentHandleRef> {
        for (_id, handle) in self.agents.read().await.iter() {
            if handle.agent_info.name == name {
                return Some(AgentHandleRef {
                    agent_info: handle.agent_info.clone(),
                    tx: handle.tx.clone(),
                    session_state: handle.session_state.clone(),
                });
            }
        }
        None
    }

    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents
            .read()
            .await
            .values()
            .map(|handle| handle.agent_info.clone())
            .collect()
    }

    pub async fn send_event_to_agent(
        &self,
        agent_id: &AgentId,
        event: AppEvent,
    ) -> anyhow::Result<()> {
        if let Some(handle) = self.agents.read().await.get(agent_id) {
            handle.tx.send(event).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent with ID {} not found", agent_id))
        }
    }

    pub async fn remove_agent(&self, agent_id: &AgentId) -> anyhow::Result<()> {
        if let Some(handle) = self.agents.write().await.remove(agent_id) {
            // Cancel the agent's task
            handle.task_handle.abort();

            // Save the session state before removing the agent
            {
                let session_state = handle.session_state.read().await;
                let session_file = SessionManager::get_session_filename(
                    if handle.agent_info.session_name == "default" {
                        None
                    } else {
                        Some(handle.agent_info.session_name.as_str())
                    },
                );
                SessionManager::save_state(&session_file, &session_state)?;
            }

            info!("Removed agent: {}", agent_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent with ID {} not found", agent_id))
        }
    }

    pub async fn switch_agent_session(
        &self,
        agent_id: &AgentId,
        session_name: Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(handle) = self.agents.read().await.get(agent_id) {
            // Save current session state
            let session_file = SessionManager::get_session_filename(
                if handle.agent_info.session_name == "default" {
                    None
                } else {
                    Some(handle.agent_info.session_name.as_str())
                },
            );
            {
                let session_state = handle.session_state.read().await;
                SessionManager::save_state(&session_file, &session_state)?;
            }

            // Load new session state
            let new_session_file = SessionManager::get_session_filename(session_name.as_deref());
            let new_session_state =
                (SessionManager::load_state(&new_session_file)?).unwrap_or_default();

            // Update agent history with new session
            {
                let mut agent_state = handle.session_state.write().await;
                *agent_state = new_session_state.clone();
            }

            // Update the agent info
            // We need to update this on the actual handle, which is in a different task
            // For now, we'll send an event to update this
            let _ = handle
                .tx
                .send(AppEvent::SwitchSession(
                    session_name.unwrap_or_else(|| "default".to_string()),
                ))
                .await;

            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent with ID {} not found", agent_id))
        }
    }
}
