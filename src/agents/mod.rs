use crate::{
    ollama::send_chat,
    types::{AppEvent, ChatMessage, Tool},
};
use reqwest::Client;
use tokio::sync::mpsc;

pub struct Agent {
    pub name: String,
    pub model: String,
    pub history: Vec<ChatMessage>,
}

impl Agent {
    pub fn new(name: &str, model: &str) -> Self {
        let system_message = "You are a helpful assistant. You have access to tools that can help you perform various tasks. Use them when appropriate.";

        Self {
            name: name.to_string(),
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
    ) -> anyhow::Result<Option<ChatMessage>> {
        let response = send_chat(client, &self.model, &self.history, tools, stream, tx).await?;

        if let Some(message) = response.clone() {
            self.add_assistant_message(message.clone());
        }

        Ok(response)
    }
}