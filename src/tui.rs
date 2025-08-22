use crate::types::AppEvent;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
    messages: Vec<String>,
    input: Input,
    is_awaiting_confirmation: bool,
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
            is_awaiting_confirmation: false,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            let messages = self.messages.clone();
            let input_val = self.input.value().to_string();
            let cursor_pos = self.input.visual_cursor();
            let is_awaiting_confirmation = self.is_awaiting_confirmation;

            self.terminal.draw(|f| {
                Tui::ui(f, &messages, &input_val, cursor_pos, is_awaiting_confirmation);
            })?;

            // Handle TUI events
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.handle_key_event(key).await? {
                        break;
                    }
                }
            }

            // Handle App events
            if let Ok(event) = self.rx.try_recv() {
                self.handle_app_event(event)?;
            }
        }
        Ok(())
    }

    fn handle_app_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::AgentStreamChunk(chunk) => {
                if let Some(last_message) = self.messages.last_mut() {
                    last_message.push_str(&chunk);
                } else {
                    self.messages.push(format!("Agent: {}", chunk));
                }
            }
            AppEvent::AgentStreamEnd => {
                // Optional: Add a newline or separator after a stream ends
            }
            AppEvent::ToolRequest(calls) => {
                let msg = format!("Agent wants to use tools: {:?}. Approve? (y/n)", calls);
                self.messages.push(msg);
                self.is_awaiting_confirmation = true;
            }
            AppEvent::ToolResult(name, result) => {
                let msg = format!("Tool '{}' result: {}", name, result);
                self.messages.push(msg);
            }
            AppEvent::Error(err) => self.messages.push(format!("Error: {}", err)),
            // Ignore AgentMessage as we are now handling streams
            AppEvent::AgentMessage(_) => {} // Ignore AgentMessage as we are now handling streams
            AppEvent::UserInput(_) => {} // Ignore UserInput as it's handled in handle_key_event
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if self.is_awaiting_confirmation {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.tx.send(AppEvent::UserInput("y".to_string())).await?;
                    self.messages.push("You: y".to_string());
                    self.is_awaiting_confirmation = false;
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.tx.send(AppEvent::UserInput("n".to_string())).await?;
                    self.messages.push("You: n".to_string());
                    self.is_awaiting_confirmation = false;
                }
                _ => {
                    // Ignore other keys
                }
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Enter => {
                let user_input = self.input.value().to_string();
                if !user_input.is_empty() {
                    self.messages.push(format!("You: {}", user_input));
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

    fn ui(
        f: &mut Frame,
        messages: &[String],
        input_val: &str,
        cursor_pos: usize,
        is_awaiting_confirmation: bool,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Min(0),      // Chat history
                    Constraint::Length(3), // User input
                ]
                .as_ref(),
            )
            .split(f.area());

        let messages = messages.join("\n");
        let chat_history = Paragraph::new(messages)
            .block(Block::default().title("Conversation").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(chat_history, chunks[0]);

        let title = if is_awaiting_confirmation {
            "Approve tool call? (y/n)"
        } else {
            "Input (Press 'q' to quit)"
        };

        let input = Paragraph::new(input_val)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(input, chunks[1]);

        if !is_awaiting_confirmation {
            f.set_cursor(
                chunks[1].x + cursor_pos as u16 + 1,
                chunks[1].y + 1,
            );
        }
    }
}