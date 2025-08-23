use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{agents::AgentId, types::ToolCall};

#[derive(Debug, Clone)]
pub enum Message {
    User(String),
    Agent(AgentId, String),
    ToolOutput(String),
    ToolConfirmation(Vec<ToolCall>),
}

impl ToString for Message {
    fn to_string(&self) -> String {
        match self {
            Message::User(s) => s.clone(),
            Message::Agent(_, s) => s.clone(),
            Message::ToolOutput(s) => s.clone(),
            Message::ToolConfirmation(calls) => {
                let mut confirmation_text = String::from("Agent wants to use the following tools:\n");
                for call in calls {
                    let args = serde_json::to_string_pretty(&call.function.arguments).unwrap_or_else(|_| "Invalid JSON".to_string());
                    confirmation_text.push_str(&format!("- {}: \n{}", call.function.name, args));
                }
                confirmation_text.push_str("Do you approve?");
                confirmation_text
            }
        }
    }
}

impl Message {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let paragraph = match self {
            Message::User(content) => Paragraph::new(content.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().bg(Color::Rgb(30, 30, 30)))
                        .title(Span::styled(
                            "You",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                )
                .wrap(Wrap { trim: true }),
            Message::Agent(_, content) => Paragraph::new(content.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .style(Style::default().bg(Color::Rgb(40, 40, 40)))
                        .title(Span::styled(
                            "Agent",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )),
                )
                .wrap(Wrap { trim: true }),
            Message::ToolOutput(content) => Paragraph::new(content.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Plain)
                        .style(Style::default().bg(Color::Rgb(20, 20, 40))) // Dark blue bg
                        .title(Span::styled(
                            "Tool Output",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )),
                )
                .wrap(Wrap { trim: true }),
            Message::ToolConfirmation(calls) => {
                let mut text = String::new();
                for call in calls {
                    text.push_str(&format!("Tool: {}
", call.function.name));
                    text.push_str(&format!(
                        "Arguments: {}
\n",
                        serde_json::to_string_pretty(&call.function.arguments)
                            .unwrap_or_else(|_| "Invalid JSON".to_string())
                    ));
                }
                Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .style(Style::default().bg(Color::Rgb(50, 20, 20))) // Dark red bg
                            .title(Span::styled(
                                "Confirmation Needed",
                                Style::default()
                                    .fg(Color::Red)
                                    .add_modifier(Modifier::BOLD),
                            )),
                    )
                    .wrap(Wrap { trim: true })
            }
        };
        frame.render_widget(paragraph, area);

    }
}
