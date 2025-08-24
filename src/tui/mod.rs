use crate::types::{AppEvent, ChatMessage, ToolApprovalResponse};
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
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

pub mod message;
use message::Message;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
    messages: Vec<Message>,
    input: Input,
    tool_calls: Vec<crate::types::ToolCall>,
    is_awaiting_confirmation: bool,
    // Track message positions for click detection
    message_positions: Vec<(usize, Rect)>, // (message_index, area)
    session_name: String,
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

        Ok(Self {
            terminal,
            rx,
            tx,
            messages,
            input: Input::default(),
            tool_calls: Vec::new(),
            is_awaiting_confirmation: false,
            message_positions: Vec::new(),
            session_name,
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
                    if chat_message.content.trim_start().starts_with("<tool_call>") {
                        // This is a thinking message, show it collapsed by default
                        messages.push(Message::Thinking(
                            crate::agents::AgentId::Ollama,
                            chat_message.content,
                            false, // collapsed by default
                        ));
                    } else {
                        // Regular assistant message
                        messages.push(Message::Agent(
                            crate::agents::AgentId::Ollama,
                            chat_message.content,
                        ));
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
            let session_name = self.session_name.clone();
            self.terminal.draw(|f| {
                self.message_positions.clear(); // Clear previous positions
                ui(
                    f,
                    &self.messages,
                    &self.input,
                    &self.tool_calls,
                    self.is_awaiting_confirmation,
                    &mut self.message_positions,
                    &session_name,
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
                    // Check if this is the start of a thinking section (starts with <think>)
                    if chunk.trim_start().starts_with("<think>") {
                        // Create a new thinking message, expanded by default
                        self.messages.push(Message::Thinking(
                            crate::agents::AgentId::Ollama,
                            chunk,
                            true,
                        ));
                    } else if let Some(last_message) = self.messages.last_mut() {
                        match last_message {
                            Message::Thinking(_, content, is_expanded) => {
                                content.push_str(&chunk);
                                // Check if this is the end of a thinking section (ends with </think>)
                                if chunk.trim_end().ends_with("</think>") {
                                    // Collapse the thinking section by default after it's complete
                                    *is_expanded = false;
                                }
                            }
                            Message::Agent(_, content) => {
                                content.push_str(&chunk);
                            }
                            _ => {
                                // Create a new agent message if the last message wasn't a thinking or agent message
                                self.messages
                                    .push(Message::Agent(crate::agents::AgentId::Ollama, chunk));
                            }
                        }
                    } else {
                        // Create a new agent message if there are no previous messages
                        self.messages
                            .push(Message::Agent(crate::agents::AgentId::Ollama, chunk));
                    }
                }
            }
            AppEvent::AgentStreamEnd => {
                // Clean up any empty agent messages that might have been created
                if let Some(Message::Agent(_, content)) = self.messages.last() {
                    if content.is_empty() {
                        self.messages.pop();
                    }
                }
            }
            AppEvent::AgentMessage(content) => {
                // Check if this is a session list message
                if content.starts_with("Available sessions:") {
                    // Show session list expanded by default
                    self.messages.push(Message::ToolOutput(content, true));
                } else {
                    self.messages
                        .push(Message::Agent(crate::agents::AgentId::Ollama, content));
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
            }
            AppEvent::SessionHistory(history) => {
                // Convert and add the session history to the messages
                let history_messages = Self::convert_history_to_messages(history);
                self.messages.extend(history_messages);
            }
            AppEvent::SwitchSession(_) => {
                // This event is sent to the orchestrator, not handled here
            }
            AppEvent::ListSessions => {
                // This event is sent to the orchestrator, not handled here
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
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
            KeyCode::Char('s') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                // Show available sessions
                self.show_sessions().await?;
            }
            KeyCode::Enter => {
                let user_input = self.input.value().to_string();
                if !user_input.is_empty() {
                    // Check if this is a session switch command
                    if user_input.starts_with("/switch ") {
                        let session_name = user_input[8..].trim().to_string();
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

    async fn show_sessions(&mut self) -> anyhow::Result<()> {
        // Send event to orchestrator to list sessions
        self.tx.send(AppEvent::ListSessions).await?;
        Ok(())
    }

    fn show_help(&mut self) {
        let help_text = r#"Available commands:
- Ctrl+q: Quit the application
- Ctrl+s: List available sessions
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
        match mouse.kind {
            MouseEventKind::Down(_) => {
                // Handle mouse click to expand/collapse messages
                self.toggle_message_expansion(mouse.column, mouse.row);
            }
            _ => {}
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

fn ui(
    f: &mut Frame,
    messages: &[Message],
    input: &Input,
    tool_calls: &[crate::types::ToolCall],
    is_awaiting_confirmation: bool,
    message_positions: &mut Vec<(usize, Rect)>,
    session_name: &str,
) {
    let input_height = if is_awaiting_confirmation {
        let mut text = String::new();
        for call in tool_calls {
            text.push_str(&format!("Tool: {}\n", call.function.name));
            text.push_str(&format!(
                "Arguments: {}\n\n",
                serde_json::to_string_pretty(&call.function.arguments)
                    .unwrap_or_else(|_| "Invalid JSON".to_string())
            ));
        }
        let width = f.area().width.saturating_sub(4) as usize;
        let mut num_lines = 0;
        for line in text.lines() {
            num_lines += (line.len() / width) + 1;
        }
        num_lines as u16 + 2
    } else if input.value().is_empty() {
        3
    } else {
        let width = f.area().width as usize - 4;
        let lines = input.value().len() / width + 1;
        lines as u16 + 2
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0), Constraint::Length(input_height)].as_ref())
        .split(f.area());

    render_chat_history(f, chunks[0], messages, message_positions, session_name);
    render_input_box(f, chunks[1], input, tool_calls, is_awaiting_confirmation);
}

fn render_input_box(
    f: &mut Frame,
    area: Rect,
    input: &Input,
    tool_calls: &[crate::types::ToolCall],
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
            text.push_str(&format!(
                "Tool: {}
",
                call.function.name
            ));
            text.push_str(&format!(
                "Arguments: {}
\n",
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
