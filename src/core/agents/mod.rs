use crate::{
    core::llm::ollama::send_chat,
    types::{AppEvent, ChatMessage, Tool},
};
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Clone)] // Added Debug and Clone for AgentId
pub enum AgentId {
    Ollama,
    // User,  // Commenting out unused variant
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentId::Ollama => write!(f, "Ollama"),
            // AgentId::User => write!(f, "User"),  // Commented out unused variant
        }
    }
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub history: Vec<ChatMessage>,
}

impl Agent {
    pub fn new(system_prompt: &str) -> Self {
        Self {
            history: vec![ChatMessage::system(system_prompt)],
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.history.push(ChatMessage::user(content));
    }

    pub fn add_assistant_message(&mut self, message: ChatMessage) {
        self.history.push(message);
    }

    pub fn update_system_prompt(&mut self, new_system_prompt: &str) {
        // Remove the old system prompt (first message if it's a system message)
        if !self.history.is_empty() && self.history[0].role == "system" {
            self.history[0] = ChatMessage::system(new_system_prompt);
        } else {
            // If there's no system message at the start, insert it
            self.history
                .insert(0, ChatMessage::system(new_system_prompt));
        }
    }

    pub async fn chat(
        &mut self,
        client: &Client,
        model: &str,
        tools: &[Tool],
        stream: bool,
        tx: mpsc::Sender<AppEvent>,
        api_base: &str,
    ) -> anyhow::Result<Option<ChatMessage>> {
        info!("=== AGENT CHAT START ===");
        info!("Agent model: {}", model);
        info!("History contains {} messages", self.history.len());
        info!("Sending {} tools to Ollama", tools.len());
        for (i, tool) in tools.iter().enumerate() {
            info!(
                "  {}. Tool: {} - {}",
                i + 1,
                tool.function.name,
                tool.truncated_description()
            );
        }
        info!("Streaming: {}", stream);

        let response = send_chat(client, model, &self.history, tools, stream, tx, api_base).await?;

        if let Some(message) = response.clone() {
            self.add_assistant_message(message.clone());
        }

        info!("=== AGENT CHAT END ===");

        Ok(response)
    }
}
