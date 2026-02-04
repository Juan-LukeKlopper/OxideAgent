use crate::{
    core::llm::client::LlmClient,
    types::{AppEvent, ChatMessage, Tool},
};
use std::fmt::Debug; // Added Debug import
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Clone)]
pub enum AgentId {
    Ollama,
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentId::Ollama => write!(f, "Ollama"),
        }
    }
}

pub struct Agent {
    pub history: Vec<ChatMessage>,
    pub llm_client: Box<dyn LlmClient>,
}

impl Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("history", &self.history)
            .finish()
    }
}

impl Agent {
    pub fn new(system_prompt: &str, llm_client: Box<dyn LlmClient>) -> Self {
        Self {
            history: vec![ChatMessage::system(system_prompt)],
            llm_client,
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
        model: &str,
        tools: &[Tool],
        stream: bool,
        tx: mpsc::Sender<AppEvent>,
    ) -> anyhow::Result<Option<ChatMessage>> {
        info!("=== AGENT CHAT START ===");
        info!("Agent model: {}", model);
        info!("History contains {} messages", self.history.len());
        info!("Sending {} tools to LLM", tools.len());
        for (i, tool) in tools.iter().enumerate() {
            info!(
                "  {}. Tool: {} - {}",
                i + 1,
                tool.function.name,
                tool.truncated_description()
            );
        }
        info!("Streaming: {}", stream);

        let response = self
            .llm_client
            .chat(model, &self.history, tools, stream, tx)
            .await?;

        if let Some(message) = response.clone() {
            self.add_assistant_message(message.clone());
        }

        info!("=== AGENT CHAT END ===");

        Ok(response)
    }
}
