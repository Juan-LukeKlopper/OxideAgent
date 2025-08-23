use crate::types::{AppEvent, ToolApprovalResponse};
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
}

impl Tui {
    pub fn new(rx: mpsc::Receiver<AppEvent>, tx: mpsc::Sender<AppEvent>) -> anyhow::Result<Self> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            rx,
            tx,
            messages: Vec::new(),
            input: Input::default(),
            tool_calls: Vec::new(),
            is_awaiting_confirmation: false,
            message_positions: Vec::new(),
        })
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
                self.messages
                    .push(Message::Agent(crate::agents::AgentId::Ollama, content));
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
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Enter => {
                let user_input = self.input.value().to_string();
                if !user_input.is_empty() {
                    self.messages.push(Message::User(user_input.clone()));
                    self.tx.send(AppEvent::UserInput(user_input)).await?;
                    self.input.reset();
                }
            }
            _ => {
                self.input.handle_event(&Event::Key(key));
            }
        }
        Ok(false)
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
) {
    let chat_history_block = Block::default().title("Conversation").borders(Borders::ALL);
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
                _ => content.clone()
            };
            
            // Count wrapped lines more accurately
            display_content.lines().map(|line| {
                if line.is_empty() {
                    1
                } else {
                    (line.chars().count() / width.max(1)) + 1
                }
            }).sum::<usize>()
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

    render_chat_history(f, chunks[0], messages, message_positions);
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
        "Input (Press 'q' to quit)"
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
