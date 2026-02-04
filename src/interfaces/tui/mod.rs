use crate::core::agents::AgentId;
use crate::core::interface::{EventEmitter, InputHandler, Interface, OutputHandler};
use crate::types::{AppEvent, ChatMessage, ToolApprovalResponse, ToolCall};
use async_trait::async_trait;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

pub mod message;
use message::Message;

// TODO: Add state for tracking selected item in switcher overlay
#[derive(Debug, Clone, Copy, PartialEq)]
enum SwitcherSelection {
    Agent(usize),   // Index of selected agent
    Session(usize), // Index of selected session
    Model(usize),   // Index of selected model
}

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
    messages: Vec<Message>,
    status_messages: Vec<String>,
    input: Input,
    tool_calls: Vec<ToolCall>,
    is_awaiting_confirmation: bool,
    show_status_overlay: bool,
    show_agent_overlay: bool,
    show_session_overlay: bool,
    show_model_overlay: bool,
    current_agent: String,
    available_agents: Vec<String>,
    current_model: String,
    available_models: Vec<String>,
    // Track message positions for click detection
    message_positions: Vec<(usize, Rect)>, // (message_index, area)
    session_name: String,
    // Track selection state for switcher navigation
    switcher_selection: SwitcherSelection,
    available_sessions: Vec<String>,
    switcher_scroll: usize,
    // Multi-agent status tracking
    agent_statuses: std::collections::HashMap<String, String>, // Agent name to status
    // Help overlay toggle
    show_help_overlay: bool,
}

impl Tui {
    pub fn new(
        rx: mpsc::Receiver<AppEvent>,
        tx: mpsc::Sender<AppEvent>,
        session_name: String,
        session_history: Vec<ChatMessage>,
        available_agents: Vec<String>,
        current_model: String,
        available_models: Vec<String>,
    ) -> anyhow::Result<Self> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // Convert session history to TUI messages
        let messages = Self::convert_history_to_messages(session_history);

        // Get available sessions from orchestrator
        let available_sessions = match crate::core::orchestrator::Orchestrator::list_sessions() {
            Ok(sessions) => sessions,
            Err(_) => vec!["default".to_string()], // Fallback to default if listing fails
        };

        Ok(Self {
            terminal,
            rx,
            tx,
            messages,
            status_messages: Vec::new(),
            input: Input::default(),
            tool_calls: Vec::new(),
            is_awaiting_confirmation: false,
            show_status_overlay: false,
            show_agent_overlay: false,
            show_session_overlay: false,
            show_model_overlay: false,
            current_agent: session_name.clone(),
            available_agents,
            current_model,
            available_models,
            message_positions: Vec::new(),
            session_name,
            // Initialize switcher selection state
            switcher_selection: SwitcherSelection::Agent(0),
            available_sessions,
            switcher_scroll: 0,
            agent_statuses: std::collections::HashMap::new(),
            show_help_overlay: false,
        })
    }

    fn convert_history_to_messages(history: Vec<ChatMessage>) -> Vec<Message> {
        let mut messages = Vec::new();

        for chat_message in history {
            match chat_message.role.as_str() {
                "user" => {
                    messages.push(Message::User(chat_message.content));
                }
                "assistant" => {
                    // Check if this is a thinking message (contains the special markers)
                    if chat_message.content.trim_start().starts_with("####") {
                        // This is a thinking message, show it collapsed by default
                        messages.push(Message::Thinking(
                            AgentId::Ollama,
                            chat_message.content,
                            false, // collapsed by default
                        ));
                    } else {
                        // Regular assistant message
                        messages.push(Message::Agent(AgentId::Ollama, chat_message.content));
                    }
                }
                "system" => {
                    // We typically don't display system messages in the UI
                    // But we could if needed
                }
                _ => {
                    // Handle any other message types
                    messages.push(Message::ToolOutput(
                        format!("Unknown message type: {}", chat_message.role),
                        false,
                    ));
                }
            }
        }

        messages
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.terminal.draw(|f| {
                self.message_positions.clear(); // Clear previous positions
                ui(
                    f,
                    &self.messages,
                    &self.input,
                    &self.tool_calls,
                    self.is_awaiting_confirmation,
                    &mut self.message_positions,
                    &self.session_name,
                    &self.status_messages,
                    self.show_status_overlay,
                    self.show_agent_overlay,
                    self.show_session_overlay,
                    self.show_model_overlay,
                    &self.current_agent,
                    &self.available_agents,
                    &self.current_model,
                    &self.available_models,
                    self.switcher_selection,
                    &self.available_sessions,
                    self.switcher_scroll,
                    &self.agent_statuses,
                    self.show_help_overlay,
                );
            })?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key_event(key).await? {
                            break;
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_event(mouse);
                    }
                    _ => {}
                }
            }

            while let Ok(event) = self.rx.try_recv() {
                self.handle_app_event(event)?;
            }
        }
        Ok(())
    }

    fn handle_app_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::AgentStreamChunk(chunk) => {
                let mut content_to_process = chunk;

                // Safety limiter to prevent infinite loops if logic is buggy
                let mut loops = 0;
                while !content_to_process.is_empty() && loops < 100 {
                    loops += 1;

                    let is_thinking =
                        matches!(self.messages.last(), Some(Message::Thinking(_, _, _)));

                    if is_thinking {
                        if let Some(Message::Thinking(_, content, _)) = self.messages.last_mut() {
                            content.push_str(&content_to_process);
                            // Check for closing tag
                            if let Some(idx) = content.find("</think>") {
                                let remainder = content[idx + 8..].to_string();
                                content.truncate(idx);

                                // Switch to Agent mode for the remainder
                                self.messages
                                    .push(Message::Agent(AgentId::Ollama, String::new()));

                                content_to_process = remainder;
                                continue;
                            } else {
                                // No closing tag found. Consumed all.
                                break;
                            }
                        } else {
                            // Should not happen given is_thinking check, but fallback
                            self.messages.push(Message::Thinking(
                                AgentId::Ollama,
                                content_to_process,
                                false,
                            ));
                            break;
                        }
                    } else {
                        // Agent Mode (default)
                        if self.messages.is_empty() {
                            self.messages
                                .push(Message::Agent(AgentId::Ollama, String::new()));
                        }

                        // Check if last message is Agent (it should be, or we just pushed one)
                        // Note: If last message is ToolOutput etc, we should start a new Agent message
                        let last_is_agent =
                            matches!(self.messages.last(), Some(Message::Agent(_, _)));
                        if !last_is_agent {
                            self.messages
                                .push(Message::Agent(AgentId::Ollama, String::new()));
                        }

                        if let Some(Message::Agent(_, content)) = self.messages.last_mut() {
                            content.push_str(&content_to_process);
                            if let Some(idx) = content.find("<think>") {
                                let remainder = content[idx + 7..].to_string();
                                content.truncate(idx);

                                // Switch to Thinking
                                // Default to collapsed (false)
                                self.messages.push(Message::Thinking(
                                    AgentId::Ollama,
                                    String::new(),
                                    false,
                                ));

                                content_to_process = remainder;
                                continue;
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
            AppEvent::AgentStreamEnd => {
                // Clean up any empty agent messages that might have been created
                if let Some(Message::Agent(_, content)) = self.messages.last()
                    && content.is_empty()
                {
                    self.messages.pop();
                }
            }
            AppEvent::AgentMessage(content) => {
                // Check if this is a session list message
                if content.starts_with("Available sessions:") {
                    // Show session list expanded by default
                    self.messages
                        .push(Message::ToolOutput(content.clone(), true));
                    // Parse session list and update available_sessions
                    if let Some(sessions_str) = content.strip_prefix("Available sessions: ") {
                        let sessions: Vec<String> =
                            sessions_str.split(", ").map(|s| s.to_string()).collect();
                        self.available_sessions = sessions;
                    }
                } else {
                    self.messages.push(Message::Agent(AgentId::Ollama, content));
                }
            }
            AppEvent::ToolRequest(calls) => {
                self.tool_calls = calls.clone();
                self.messages.push(Message::ToolConfirmation(calls));
                self.is_awaiting_confirmation = true;
            }
            AppEvent::ToolResult(name, result) => {
                let msg = format!("Tool '{}' result: {}", name, result);
                self.messages.push(Message::ToolOutput(msg, false)); // Collapsed by default
            }
            AppEvent::Error(err) => {
                self.messages
                    .push(Message::ToolOutput(format!("Error: {}", err), false)); // Collapsed by default
            }
            AppEvent::UserInput(_) => {}
            AppEvent::ToolApproval(_) => {}
            AppEvent::SessionSwitched(session_name) => {
                // Update the session name
                self.session_name = session_name;
                // Clear the current messages
                self.messages.clear();
                // Add a message to indicate the session switch
                self.messages.push(Message::ToolOutput(
                    format!(
                        "Switched to session: {}\nSession history has been restored.",
                        self.session_name
                    ),
                    false,
                ));
                // Refresh session list to include the new session
                let _ = self.tx.try_send(AppEvent::RefreshSessions);
            }
            AppEvent::SessionList(sessions) => {
                // Update available sessions without displaying a message
                self.available_sessions = sessions;
            }
            AppEvent::SessionHistory(history) => {
                // Clear existing messages first before loading new session
                self.messages.clear();
                // Convert and add the session history to the messages
                let history_messages = Self::convert_history_to_messages(history);
                self.messages.extend(history_messages);
            }
            AppEvent::ContinueConversation => {
                // This event is handled by the orchestrator, not the TUI
                // The TUI doesn't need to do anything special here
            }
            AppEvent::SwitchSession(_) => {
                // This event is sent to the orchestrator, not handled here
            }
            AppEvent::SwitchAgent(agent_name, _) => {
                // Update the current agent name
                self.current_agent = agent_name.clone();
                // Add a message to indicate the agent switch
                self.messages.push(Message::ToolOutput(
                    format!("Switched to agent: {}", agent_name),
                    false,
                ));
            }
            AppEvent::SwitchModel(model_name) => {
                // Update the current model name
                self.current_model = model_name.clone();
                // Add a message to indicate the model switch
                self.messages.push(Message::ToolOutput(
                    format!("Switched to model: {}", model_name),
                    false,
                ));
            }
            AppEvent::RefreshSessions => {
                // This event is sent to the orchestrator, not handled here
            }
            AppEvent::ListSessions => {
                // This event is sent to the orchestrator, not handled here
            }
            AppEvent::AgentStatusUpdate(agent_name, status) => {
                // Update the agent status in our local map
                self.agent_statuses.insert(agent_name, status);
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        // TODO: Handle navigation in switcher overlay
        if self.show_agent_overlay || self.show_session_overlay || self.show_model_overlay {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    self.show_agent_overlay = false;
                    self.show_session_overlay = false;
                    self.show_model_overlay = false;
                    return Ok(false);
                }
                KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    // Close agent overlay if open
                    if self.show_agent_overlay {
                        self.show_agent_overlay = false;
                        return Ok(false);
                    }
                }
                KeyCode::Char('s') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    // Close session overlay if open
                    if self.show_session_overlay {
                        self.show_session_overlay = false;
                        return Ok(false);
                    }
                }
                KeyCode::Char('m') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    // Close model overlay if open
                    if self.show_model_overlay {
                        self.show_model_overlay = false;
                        return Ok(false);
                    }
                }
                KeyCode::Esc => {
                    self.show_agent_overlay = false;
                    self.show_session_overlay = false;
                    self.show_model_overlay = false;
                    return Ok(false);
                }
                KeyCode::Up => {
                    self.navigate_switcher_up();
                    return Ok(false);
                }
                KeyCode::Down => {
                    self.navigate_switcher_down();
                    return Ok(false);
                }
                KeyCode::Enter => {
                    self.select_switcher_item().await?;
                    return Ok(false);
                }
                _ => {}
            }
        }

        if self.is_awaiting_confirmation {
            match key.code {
                KeyCode::Char('1') => {
                    self.tx
                        .send(AppEvent::ToolApproval(ToolApprovalResponse::Allow))
                        .await?;
                    self.messages.push(Message::User("Allowed".to_string()));
                    self.is_awaiting_confirmation = false;
                }
                KeyCode::Char('2') => {
                    self.tx
                        .send(AppEvent::ToolApproval(ToolApprovalResponse::AlwaysAllow))
                        .await?;
                    self.messages
                        .push(Message::User("Always Allowed".to_string()));
                    self.is_awaiting_confirmation = false;
                }
                KeyCode::Char('3') => {
                    self.tx
                        .send(AppEvent::ToolApproval(
                            ToolApprovalResponse::AlwaysAllowSession,
                        ))
                        .await?;
                    self.messages
                        .push(Message::User("Always Allowed for Session".to_string()));
                    self.is_awaiting_confirmation = false;
                }
                KeyCode::Char('4') => {
                    self.tx
                        .send(AppEvent::ToolApproval(ToolApprovalResponse::Deny))
                        .await?;
                    self.messages.push(Message::User("Denied".to_string()));
                    self.is_awaiting_confirmation = false;
                }
                _ => {}
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                return Ok(true);
            }
            KeyCode::Char('o') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                // Toggle help overlay
                self.show_help_overlay = !self.show_help_overlay;
            }
            KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                // Toggle agent overlay
                self.show_agent_overlay = !self.show_agent_overlay;
                self.show_session_overlay = false;
                self.show_model_overlay = false;

                // Reset selection when opening
                if self.show_agent_overlay {
                    self.switcher_selection = SwitcherSelection::Agent(0);
                    self.switcher_scroll = 0;
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                // Toggle session overlay
                self.show_session_overlay = !self.show_session_overlay;
                self.show_agent_overlay = false;
                self.show_model_overlay = false;

                // Reset selection when opening
                if self.show_session_overlay {
                    self.switcher_selection = SwitcherSelection::Session(0);
                    self.switcher_scroll = 0;
                    // Request updated session list
                    self.tx.send(AppEvent::RefreshSessions).await?;
                }
            }
            KeyCode::Char('l') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                // Toggle model overlay
                self.show_model_overlay = !self.show_model_overlay;
                self.show_agent_overlay = false;
                self.show_session_overlay = false;

                // Reset selection when opening
                if self.show_model_overlay {
                    self.switcher_selection = SwitcherSelection::Model(0);
                    self.switcher_scroll = 0;
                }
            }
            KeyCode::Enter => {
                let user_input = self.input.value().to_string();
                if !user_input.is_empty() {
                    // Check if this is a session switch command
                    if let Some(stripped) = user_input.strip_prefix("/switch ") {
                        let session_name = stripped.trim().to_string();
                        self.tx.send(AppEvent::SwitchSession(session_name)).await?;
                        self.messages.push(Message::User(user_input.clone()));
                    } else if let Some(stripped) = user_input.strip_prefix("/model ") {
                        let model_name = stripped.trim().to_string();
                        self.tx.send(AppEvent::SwitchModel(model_name)).await?;
                        self.messages.push(Message::User(user_input.clone()));
                    } else {
                        self.messages.push(Message::User(user_input.clone()));
                        self.tx.send(AppEvent::UserInput(user_input)).await?;
                    }
                    self.input.reset();
                }
            }
            _ => {
                self.input.handle_event(&Event::Key(key));
            }
        }
        Ok(false)
    }

    // Implement switcher navigation methods
    fn navigate_switcher_up(&mut self) {
        let panel_height = 13; // Height of inner content area (adjusted for title/footer)
        match self.switcher_selection {
            SwitcherSelection::Agent(idx) => {
                let len = self.available_agents.len();
                if len == 0 {
                    return;
                }
                let new_idx = if idx > 0 { idx - 1 } else { len - 1 };
                self.switcher_selection = SwitcherSelection::Agent(new_idx);
                self.adjust_scroll_up(new_idx, panel_height);
            }
            SwitcherSelection::Session(idx) => {
                let len = self.available_sessions.len();
                if len == 0 {
                    return;
                }
                let new_idx = if idx > 0 { idx - 1 } else { len - 1 };
                self.switcher_selection = SwitcherSelection::Session(new_idx);
                self.adjust_scroll_up(new_idx, panel_height);
            }
            SwitcherSelection::Model(idx) => {
                let len = self.available_models.len();
                if len == 0 {
                    return;
                }
                let new_idx = if idx > 0 { idx - 1 } else { len - 1 };
                self.switcher_selection = SwitcherSelection::Model(new_idx);
                self.adjust_scroll_up(new_idx, panel_height);
            }
        }
    }

    fn navigate_switcher_down(&mut self) {
        let panel_height = 13; // Height of inner content area
        match self.switcher_selection {
            SwitcherSelection::Agent(idx) => {
                let len = self.available_agents.len();
                if len == 0 {
                    return;
                }
                let new_idx = if idx + 1 < len { idx + 1 } else { 0 };
                self.switcher_selection = SwitcherSelection::Agent(new_idx);
                self.adjust_scroll_down(new_idx, panel_height);
            }
            SwitcherSelection::Session(idx) => {
                let len = self.available_sessions.len();
                if len == 0 {
                    return;
                }
                let new_idx = if idx + 1 < len { idx + 1 } else { 0 };
                self.switcher_selection = SwitcherSelection::Session(new_idx);
                self.adjust_scroll_down(new_idx, panel_height);
            }
            SwitcherSelection::Model(idx) => {
                let len = self.available_models.len();
                if len == 0 {
                    return;
                }
                let new_idx = if idx + 1 < len { idx + 1 } else { 0 };
                self.switcher_selection = SwitcherSelection::Model(new_idx);
                self.adjust_scroll_down(new_idx, panel_height);
            }
        }
    }

    fn adjust_scroll_up(&mut self, new_idx: usize, panel_height: usize) {
        if new_idx < self.switcher_scroll {
            self.switcher_scroll = new_idx;
        } else if new_idx >= self.switcher_scroll + panel_height {
            // Wrapped around to bottom
            self.switcher_scroll = new_idx.saturating_sub(panel_height) + 1;
        }
    }

    fn adjust_scroll_down(&mut self, new_idx: usize, panel_height: usize) {
        if new_idx >= self.switcher_scroll + panel_height {
            self.switcher_scroll = new_idx - panel_height + 1;
        } else if new_idx < self.switcher_scroll {
            // Wrapped around to top
            self.switcher_scroll = 0;
        }
    }

    async fn select_switcher_item(&mut self) -> anyhow::Result<()> {
        match self.switcher_selection {
            SwitcherSelection::Agent(idx) => {
                if idx < self.available_agents.len() {
                    let selected_agent = &self.available_agents[idx];
                    if selected_agent != &self.current_agent {
                        let agent_type_name = if selected_agent.contains("qwen") {
                            "Qwen"
                        } else if selected_agent.contains("llama") {
                            "Llama"
                        } else if selected_agent.contains("granite") {
                            "Granite"
                        } else {
                            selected_agent
                        };

                        // Send switch agent event to orchestrator
                        self.tx
                            .send(AppEvent::SwitchAgent(
                                agent_type_name.to_string(),
                                self.session_name.clone(),
                            ))
                            .await?;
                        self.messages
                            .push(Message::User(format!("/switch agent {}", selected_agent)));
                        // Update the current agent locally
                        self.current_agent = selected_agent.clone();
                    }
                }
            }
            SwitcherSelection::Session(idx) => {
                if idx < self.available_sessions.len() {
                    let selected_session = &self.available_sessions[idx];
                    if selected_session != &self.session_name {
                        // Send switch session event
                        self.tx
                            .send(AppEvent::SwitchSession(selected_session.clone()))
                            .await?;
                        self.messages
                            .push(Message::User(format!("/switch {}", selected_session)));
                    }
                }
            }
            SwitcherSelection::Model(idx) => {
                if idx < self.available_models.len() {
                    let selected_model = &self.available_models[idx];
                    if selected_model != &self.current_model {
                        // Send switch model event
                        self.tx
                            .send(AppEvent::SwitchModel(selected_model.clone()))
                            .await?;
                        self.messages
                            .push(Message::User(format!("/model {}", selected_model)));
                    }
                }
            }
        }
        self.show_agent_overlay = false;
        self.show_session_overlay = false;
        self.show_model_overlay = false;
        Ok(())
    }

    #[allow(dead_code)]
    fn show_help(&mut self) {
        let help_text = r#"Available commands:
- Ctrl+q: Quit the application
- Ctrl+a: Toggle agent/session switcher
- Ctrl+o: Show this help message
- /switch <session_name>: Switch to a different session
- Type your message and press Enter to chat

Tool approval options (when prompted):
- 1: Allow tool execution
- 2: Always allow this tool
- 3: Always allow this tool for this session
- 4: Deny tool execution"#;

        self.messages
            .push(Message::ToolOutput(help_text.to_string(), true));
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if let MouseEventKind::Down(_) = mouse.kind {
            // Handle mouse click to expand/collapse messages
            self.toggle_message_expansion(mouse.column, mouse.row);
        }
    }

    fn toggle_message_expansion(&mut self, column: u16, row: u16) {
        // Find which message was clicked based on position
        for (message_index, area) in &self.message_positions {
            if column >= area.x
                && column < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
            {
                // Found the clicked message
                if let Some(message) = self.messages.get_mut(*message_index) {
                    match message {
                        Message::Thinking(_, _, is_expanded) => {
                            *is_expanded = !*is_expanded;
                        }
                        Message::ToolOutput(_, is_expanded) => {
                            *is_expanded = !*is_expanded;
                        }
                        _ => {}
                    }
                }
                break;
            }
        }
    }

    pub fn restore(&mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

#[async_trait]
impl InputHandler for Tui {
    async fn handle_input(&mut self, input: String) -> anyhow::Result<()> {
        // Send the input to the orchestrator
        self.tx.send(AppEvent::UserInput(input)).await?;
        Ok(())
    }
}

#[async_trait]
impl OutputHandler for Tui {
    async fn send_output(&mut self, output: AppEvent) -> anyhow::Result<()> {
        // Handle the output event
        self.handle_app_event(output)
    }
}

impl EventEmitter for Tui {
    fn get_event_sender(&self) -> mpsc::Sender<AppEvent> {
        self.tx.clone()
    }

    fn get_event_receiver(&mut self) -> mpsc::Receiver<AppEvent> {
        // This is a bit tricky since we already have a receiver
        // In a real implementation, we might want to restructure this
        // For now, we'll just return a dummy channel
        let (_tx, rx) = mpsc::channel(1);
        // We're not using this in the current implementation, so it's fine
        rx
    }
}

#[async_trait]
impl Interface for Tui {
    async fn init(&mut self) -> anyhow::Result<()> {
        // The TUI is already initialized in the new() function
        Ok(())
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        // Run the TUI event loop
        self.run().await
    }

    async fn cleanup(&mut self) -> anyhow::Result<()> {
        // Restore the terminal
        self.restore()
    }

    fn get_session_history(&self) -> Vec<ChatMessage> {
        // Convert TUI messages back to ChatMessage format
        let mut history = Vec::new();
        for message in &self.messages {
            match message {
                Message::User(content) => {
                    history.push(ChatMessage {
                        role: "user".to_string(),
                        content: content.clone(),
                        tool_calls: None,
                    });
                }
                Message::Agent(_, content) => {
                    history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: content.clone(),
                        tool_calls: None,
                    });
                }
                Message::Thinking(_, content, _) => {
                    history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: content.clone(),
                        tool_calls: None,
                    });
                }
                Message::ToolOutput(_content, _) => {
                    // Tool outputs are typically not part of the session history
                    // But we might want to include some of them
                }
                Message::ToolConfirmation(_) => {
                    // Tool confirmations are not part of the session history
                }
            }
        }
        history
    }

    fn get_session_name(&self) -> String {
        self.session_name.clone()
    }
}

fn render_chat_history(
    f: &mut Frame,
    area: Rect,
    messages: &[Message],
    message_positions: &mut Vec<(usize, Rect)>,
    session_name: &str,
) {
    let chat_history_block = Block::default()
        .title(format!("Conversation - Session: {}", session_name))
        .borders(Borders::ALL);
    let inner_chat_area = chat_history_block.inner(area);
    f.render_widget(chat_history_block, area);

    let mut y_offset = 0;
    let _message_index = messages.len();

    for (idx, msg) in messages.iter().enumerate().rev() {
        let content = msg.to_string();
        let width = inner_chat_area.width as usize;
        // Calculate height based on actual rendered content
        let line_count = if width > 0 {
            // For expandable messages, calculate based on displayed content
            let display_content = match msg {
                Message::Thinking(_, content, is_expanded) => {
                    if *is_expanded {
                        // Expanded thinking - show full content
                        content.clone()
                    } else {
                        // Collapsed thinking - show just a preview
                        let lines: Vec<&str> = content.lines().collect();
                        if lines.len() > 3 {
                            format!("{}\n{}\n{}...", lines[0], lines[1], lines[2])
                        } else {
                            lines.first().unwrap_or(&"Thinking...").to_string()
                        }
                    }
                }
                Message::ToolOutput(content, is_expanded) => {
                    if *is_expanded {
                        // Expanded tool output - show full content
                        content.clone()
                    } else {
                        // Collapsed tool output - show just a preview
                        let lines: Vec<&str> = content.lines().collect();
                        if lines.len() > 3 {
                            format!("{}\n{}\n{}...", lines[0], lines[1], lines[2])
                        } else {
                            content.clone()
                        }
                    }
                }
                _ => content.clone(),
            };

            // Count wrapped lines more accurately
            display_content
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        1
                    } else {
                        (line.chars().count() / width.max(1)) + 1
                    }
                })
                .sum::<usize>()
        } else {
            content.lines().count()
        };
        let height = line_count as u16 + 2; // +2 for borders

        if y_offset + height < inner_chat_area.height {
            let msg_area = Rect::new(
                inner_chat_area.x,
                inner_chat_area.y + inner_chat_area.height - y_offset - height,
                inner_chat_area.width,
                height,
            );

            // Store the position of this message for click detection
            message_positions.push((idx, msg_area));

            msg.render(f, msg_area);
            y_offset += height;
        } else {
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn ui(
    f: &mut Frame,
    messages: &[Message],
    input: &Input,
    tool_calls: &[ToolCall],
    is_awaiting_confirmation: bool,
    message_positions: &mut Vec<(usize, Rect)>,
    session_name: &str,
    _status_messages: &[String],
    _show_status_overlay: bool,
    show_agent_overlay: bool,
    show_session_overlay: bool,
    show_model_overlay: bool,
    current_agent: &str,
    available_agents: &[String],
    current_model: &str,
    available_models: &[String],
    switcher_selection: SwitcherSelection,
    available_sessions: &[String],
    switcher_scroll: usize,
    agent_statuses: &std::collections::HashMap<String, String>,
    show_help_overlay: bool,
) {
    if show_help_overlay {
        let area = centered_rect(60, 50, f.area());
        let help_text = vec![
            Line::from(vec![Span::raw("Available commands:")]),
            Line::from(vec![Span::raw("- Ctrl+q: Quit application")]),
            Line::from(vec![Span::raw("- Ctrl+a: Open Agent Switcher")]),
            Line::from(vec![Span::raw("- Ctrl+s: Open Session Switcher")]),
            Line::from(vec![Span::raw("- Ctrl+l: Open Model Switcher")]),
            Line::from(vec![Span::raw("- Ctrl+o: Toggle this help")]),
            Line::from(vec![Span::raw("- /switch <session_name>: Switch session")]),
            Line::from(vec![Span::raw("- /model <model_name>: Switch model")]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::raw("Tool Approvals:")]),
            Line::from(vec![Span::raw("- 1: Allow once")]),
            Line::from(vec![Span::raw("- 2: Always allow")]),
            Line::from(vec![Span::raw("- 3: Always allow for session")]),
            Line::from(vec![Span::raw("- 4: Deny")]),
        ];

        // Render background (chat history) dimmed or as is
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(f.area());

        render_chat_history(f, chunks[0], messages, message_positions, session_name);
        render_input_box(f, chunks[1], input, tool_calls, is_awaiting_confirmation);

        // Render help popup
        let block = Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let paragraph = Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, area); // Clear area behind popup
        f.render_widget(paragraph, area);
    } else if show_agent_overlay || show_session_overlay || show_model_overlay {
        // Multipane layout with switcher panel
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Min(0),     // Chat history (shrunk)
                    Constraint::Length(15), // Switcher panel
                    Constraint::Length(3),  // Input box
                ]
                .as_ref(),
            )
            .split(f.area());

        render_chat_history(f, chunks[0], messages, message_positions, session_name);
        render_switcher_panel(
            f,
            chunks[1],
            current_agent,
            available_agents,
            current_model,
            available_models,
            session_name,
            available_sessions,
            switcher_selection,
            switcher_scroll,
            agent_statuses,
        );
        render_input_box(f, chunks[2], input, tool_calls, is_awaiting_confirmation);
    } else {
        // Normal layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(f.area());

        render_chat_history(f, chunks[0], messages, message_positions, session_name);
        render_input_box(f, chunks[1], input, tool_calls, is_awaiting_confirmation);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_switcher_panel(
    f: &mut Frame,
    area: Rect,
    current_agent: &str,
    available_agents: &[String],
    current_model: &str,
    available_models: &[String],
    current_session: &str,
    available_sessions: &[String],
    switcher_selection: SwitcherSelection,
    switcher_scroll: usize,
    agent_statuses: &std::collections::HashMap<String, String>,
) {
    // Temporarily create a block to get inner_area for calculations
    let temp_block = Block::default().borders(Borders::ALL);
    let inner_area = temp_block.inner(area);

    let (title, content_string, total_count) = match switcher_selection {
        SwitcherSelection::Agent(selected_idx) => {
            let mut text = String::from("Available Agents:\n");
            for (offset, agent) in available_agents.iter().enumerate() {
                if offset < switcher_scroll
                    || offset >= switcher_scroll + inner_area.height as usize - 1
                {
                    // -1 for potential footer
                    continue;
                }
                let status = agent_statuses
                    .get(agent)
                    .cloned()
                    .unwrap_or_else(|| "Idle".to_string());
                let is_selected = offset == selected_idx;
                let is_current = agent == current_agent;

                let marker = if is_selected { "->" } else { "  " };
                let current_marker = if is_current { " (current)" } else { "" };
                let select_marker = if is_selected { " (selected)" } else { "" };

                text.push_str(&format!(
                    " {} [{}] [{}] {}{}\n",
                    marker, agent, status, current_marker, select_marker
                ));
            }
            (
                "Switch Agent (Ctrl+A / Esc to close)",
                text,
                available_agents.len(),
            )
        }
        SwitcherSelection::Session(selected_idx) => {
            let mut text = String::from("Available Sessions:\n");
            for (offset, session) in available_sessions.iter().enumerate() {
                if offset < switcher_scroll
                    || offset >= switcher_scroll + inner_area.height as usize - 1
                {
                    // -1 for potential footer
                    continue;
                }
                let is_selected = offset == selected_idx;
                let is_current = session == current_session;

                let marker = if is_selected { "->" } else { "  " };
                let current_marker = if is_current { " (current)" } else { "" };
                let select_marker = if is_selected { " (selected)" } else { "" };

                text.push_str(&format!(
                    " {} [{}]{}{}\n",
                    marker, session, current_marker, select_marker
                ));
            }
            (
                "Switch Session (Ctrl+S / Esc to close)",
                text,
                available_sessions.len(),
            )
        }
        SwitcherSelection::Model(selected_idx) => {
            let mut text = String::from("Available Models:\n");
            for (offset, model) in available_models.iter().enumerate() {
                if offset < switcher_scroll
                    || offset >= switcher_scroll + inner_area.height as usize - 1
                {
                    // -1 for potential footer
                    continue;
                }
                let is_selected = offset == selected_idx;
                let is_current = model == current_model;

                let marker = if is_selected { "->" } else { "  " };
                let current_marker = if is_current { " (current)" } else { "" };
                let select_marker = if is_selected { " (selected)" } else { "" };

                text.push_str(&format!(
                    " {} [{}]{}{}\n",
                    marker, model, current_marker, select_marker
                ));
            }
            (
                "Switch Model (Ctrl+L / Esc to close)",
                text,
                available_models.len(),
            )
        }
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(20, 20, 35)));

    f.render_widget(block, area);

    let mut final_content = content_string;
    if total_count > inner_area.height as usize {
        let footer = format!(
            "\n... {}/{} ... Use Up/Down arrows to navigate, Enter to select.",
            switcher_scroll + inner_area.height as usize,
            total_count
        );
        final_content.push_str(&footer);
    } else {
        final_content.push_str("\nUse Up/Down arrows to navigate, Enter to select.\n");
    }

    let paragraph = Paragraph::new(final_content)
        .wrap(Wrap { trim: false }) // No trim to keep indentation
        .style(Style::default().bg(Color::Rgb(20, 20, 35)).fg(Color::White));
    f.render_widget(paragraph, inner_area);
}

fn render_input_box(
    f: &mut Frame,
    area: Rect,
    input: &Input,
    tool_calls: &[ToolCall],
    is_awaiting_confirmation: bool,
) {
    let title = if is_awaiting_confirmation {
        "Approve tool call? (1: Allow, 2: Always Allow, 3: Always Allow (Session), 4: Deny)"
    } else {
        "Input (Press Ctrl+q to quit, Ctrl+o for help)"
    };

    let block = Block::default().title(title).borders(Borders::ALL);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if is_awaiting_confirmation {
        let mut text = String::new();
        for call in tool_calls {
            text.push_str(&format!("Tool: {}\n", call.function.name));
            text.push_str(&format!(
                "Arguments: {}\n\n",
                serde_json::to_string_pretty(&call.function.arguments)
                    .unwrap_or_else(|_| "Invalid JSON".to_string())
            ));
        }
        let confirmation_paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
        f.render_widget(confirmation_paragraph, inner_area);
    } else {
        let input_paragraph = Paragraph::new(input.value()).wrap(Wrap { trim: false });
        f.render_widget(input_paragraph, inner_area);
        let cursor_x = inner_area.x + (input.visual_cursor() as u16 % inner_area.width);
        let cursor_y = inner_area.y + (input.visual_cursor() as u16 / inner_area.width);
        f.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
