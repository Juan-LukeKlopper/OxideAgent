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
    widgets::{Block, Borders, Paragraph, Wrap},
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
    show_switcher_overlay: bool,
    current_agent: String,
    available_agents: Vec<String>,
    // Track message positions for click detection
    message_positions: Vec<(usize, Rect)>, // (message_index, area)
    session_name: String,
    // Track selection state for switcher navigation
    switcher_selection: SwitcherSelection,
    available_sessions: Vec<String>,
}

impl Tui {
    pub fn new(
        rx: mpsc::Receiver<AppEvent>,
        tx: mpsc::Sender<AppEvent>,
        session_name: String,
        session_history: Vec<ChatMessage>,
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
            show_switcher_overlay: false,
            current_agent: session_name.clone(),
            available_agents: vec![
                "Qwen".to_string(),
                "Llama".to_string(),
                "Granite".to_string(),
            ],
            message_positions: Vec::new(),
            session_name,
            // Initialize switcher selection state
            switcher_selection: SwitcherSelection::Agent(0),
            available_sessions,
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
                    self.show_switcher_overlay,
                    &self.current_agent,
                    &self.available_agents,
                    self.switcher_selection,
                    &self.available_sessions,
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

            if let Ok(event) = self.rx.try_recv() {
                self.handle_app_event(event)?;
            }
        }
        Ok(())
    }

    fn handle_app_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::AgentStreamChunk(chunk) => {
                if !chunk.is_empty() {
                    // Check if this is the start of a thinking section (starts with ####)
                    if chunk.trim_start().starts_with("####") {
                        // Create a new thinking message, expanded by default
                        self.messages
                            .push(Message::Thinking(AgentId::Ollama, chunk, true));
                    } else if let Some(last_message) = self.messages.last_mut() {
                        match last_message {
                            Message::Thinking(_, content, is_expanded) => {
                                content.push_str(&chunk);
                                // Check if this is the end of a thinking section (ends with ####)
                                if chunk.trim_end().ends_with("####") {
                                    // Collapse the thinking section by default after it's complete
                                    *is_expanded = false;
                                }
                            }
                            Message::Agent(_, content) => {
                                content.push_str(&chunk);
                            }
                            _ => {
                                // Create a new agent message if the last message wasn't a thinking or agent message
                                self.messages.push(Message::Agent(AgentId::Ollama, chunk));
                            }
                        }
                    } else {
                        // Create a new agent message if there are no previous messages
                        self.messages.push(Message::Agent(AgentId::Ollama, chunk));
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
                // Convert and add the session history to the messages
                let history_messages = Self::convert_history_to_messages(history);
                self.messages.extend(history_messages);
            }
            AppEvent::SwitchSession(_) => {
                // This event is sent to the orchestrator, not handled here
            }
            AppEvent::SwitchAgent(agent_name) => {
                // Update the current agent name
                self.current_agent = agent_name.clone();
                // Add a message to indicate the agent switch
                self.messages.push(Message::ToolOutput(
                    format!("Switched to agent: {}", agent_name),
                    false,
                ));
            }
            AppEvent::RefreshSessions => {
                // This event is sent to the orchestrator, not handled here
            }
            AppEvent::ListSessions => {
                // This event is sent to the orchestrator, not handled here
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        // TODO: Handle navigation in switcher overlay
        if self.show_switcher_overlay {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    self.show_switcher_overlay = false;
                    return Ok(false);
                }
                KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    self.show_switcher_overlay = false;
                    return Ok(false);
                }
                KeyCode::Esc => {
                    self.show_switcher_overlay = false;
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
                // Show help
                self.show_help();
            }
            KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                // Toggle switcher overlay
                self.show_switcher_overlay = !self.show_switcher_overlay;
                // Reset selection when opening
                if self.show_switcher_overlay {
                    self.switcher_selection = SwitcherSelection::Agent(0);
                    // Request updated session list without displaying response
                    self.tx.send(AppEvent::RefreshSessions).await?;
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
        match self.switcher_selection {
            SwitcherSelection::Agent(idx) => {
                if idx > 0 {
                    self.switcher_selection = SwitcherSelection::Agent(idx - 1);
                } else {
                    // Move to last session
                    self.switcher_selection =
                        SwitcherSelection::Session(self.available_sessions.len().saturating_sub(1));
                }
            }
            SwitcherSelection::Session(idx) => {
                if idx > 0 {
                    self.switcher_selection = SwitcherSelection::Session(idx - 1);
                } else {
                    // Move to last agent
                    self.switcher_selection =
                        SwitcherSelection::Agent(self.available_agents.len().saturating_sub(1));
                }
            }
        }
    }

    fn navigate_switcher_down(&mut self) {
        match self.switcher_selection {
            SwitcherSelection::Agent(idx) => {
                if idx + 1 < self.available_agents.len() {
                    self.switcher_selection = SwitcherSelection::Agent(idx + 1);
                } else {
                    // Move to first session
                    self.switcher_selection = SwitcherSelection::Session(0);
                }
            }
            SwitcherSelection::Session(idx) => {
                if idx + 1 < self.available_sessions.len() {
                    self.switcher_selection = SwitcherSelection::Session(idx + 1);
                } else {
                    // Move to first agent
                    self.switcher_selection = SwitcherSelection::Agent(0);
                }
            }
        }
    }

    async fn select_switcher_item(&mut self) -> anyhow::Result<()> {
        match self.switcher_selection {
            SwitcherSelection::Agent(idx) => {
                if idx < self.available_agents.len() {
                    let selected_agent = &self.available_agents[idx];
                    if selected_agent != &self.current_agent {
                        // Send switch agent event to orchestrator
                        self.tx
                            .send(AppEvent::SwitchAgent(selected_agent.clone()))
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
        }
        self.show_switcher_overlay = false;
        Ok(())
    }

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
    show_switcher_overlay: bool,
    current_agent: &str,
    available_agents: &[String],
    switcher_selection: SwitcherSelection,
    available_sessions: &[String],
) {
    if show_switcher_overlay {
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
            session_name,
            available_sessions,
            switcher_selection,
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

fn render_switcher_panel(
    f: &mut Frame,
    area: Rect,
    current_agent: &str,
    available_agents: &[String],
    current_session: &str,
    available_sessions: &[String],
    switcher_selection: SwitcherSelection,
) {
    let block = Block::default()
        .title("Switch Agent/Session (Press Ctrl+a or Esc to close)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(20, 20, 35))); // Solid dark background

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Render available agents
    let mut text = String::new();
    text.push_str("Available Agents:\n");

    for (idx, agent) in available_agents.iter().enumerate() {
        let is_selected = match switcher_selection {
            SwitcherSelection::Agent(selected_idx) => selected_idx == idx,
            _ => false,
        };

        if agent == current_agent {
            if is_selected {
                // Highlight current selected agent with bright color
                text.push_str(&format!("  -> [{}] (current, selected)\n", agent));
            } else {
                // Highlight current agent
                text.push_str(&format!("     {} (current)\n", agent));
            }
        } else if is_selected {
            // Highlight selected agent with bright color
            text.push_str(&format!("  -> [{}]\n", agent));
        } else {
            text.push_str(&format!("     {}\n", agent));
        }
    }

    text.push_str("\nAvailable Sessions:\n");

    // Render available sessions
    for (idx, session) in available_sessions.iter().enumerate() {
        let is_selected = match switcher_selection {
            SwitcherSelection::Session(selected_idx) => selected_idx == idx,
            _ => false,
        };

        if session == current_session {
            if is_selected {
                // Highlight current selected session with bright color
                text.push_str(&format!("  -> [{}] (current, selected)\n", session));
            } else {
                // Highlight current session
                text.push_str(&format!("     {} (current)\n", session));
            }
        } else if is_selected {
            // Highlight selected session with bright color\n
            text.push_str(&format!("  -> [{}]\\n", session));
        } else {
            text.push_str(&format!("     {}\\n", session));
        }
    }

    text.push_str("\nUse Up/Down arrows to navigate, Enter to select.\n");

    // Create a paragraph with high-contrast text on the solid background
    let switcher_paragraph = Paragraph::new(text).wrap(Wrap { trim: true }).style(
        Style::default()
            .bg(Color::Rgb(20, 20, 35)) // Match background color for consistency
            .fg(Color::White),
    ); // High-contrast white text
    f.render_widget(switcher_paragraph, inner_area);
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
