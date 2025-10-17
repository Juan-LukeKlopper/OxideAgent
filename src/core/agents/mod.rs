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
    pub model: String,
    pub history: Vec<ChatMessage>,
}

impl Agent {
    pub fn new(_name: &str, model: &str) -> Self {
        let system_message = "You are a helpful assistant. You have access to tools that can help you perform various tasks. Use them when appropriate.";

        Self {
            model: model.to_string(),
            history: vec![ChatMessage::system(system_message)],
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.history.push(ChatMessage::user(content));
    }

    pub fn add_assistant_message(&mut self, message: ChatMessage) {
        self.history.push(message);
    }

    pub async fn chat(
        &mut self,
        client: &Client,
        tools: &[Tool],
        stream: bool,
        tx: mpsc::Sender<AppEvent>,
        api_base: &str,
    ) -> anyhow::Result<Option<ChatMessage>> {
        info!("=== AGENT CHAT START ===");
        info!("Agent model: {}", self.model);
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

        let response = send_chat(
            client,
            &self.model,
            &self.history,
            tools,
            stream,
            tx,
            api_base,
        )
        .await?;

        if let Some(message) = response.clone() {
            self.add_assistant_message(message.clone());
        }

        info!("=== AGENT CHAT END ===");

        Ok(response)
    }
}
