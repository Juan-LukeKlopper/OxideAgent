use crate::ollama::send_chat;
use crate::types::ChatMessage;
use reqwest::Client;
use std::io::{self, Write};

pub struct Agent {
    pub name: String,
    pub model: String,
    pub history: Vec<ChatMessage>,
}

impl Agent {
    pub fn new(name: &str, model: &str) -> Self {
        let system_message = r#"You are a helpful assistant.
You have access to the following tools. To use a tool, respond with a JSON object with the following format:
{"tool": "write_file", "path": "<filename>", "content": "<file_content>"}
"#;
        Self {
            name: name.to_string(),
            model: model.to_string(),
            history: vec![ChatMessage::system(system_message)],
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.history.push(ChatMessage::user(content));
    }

    pub fn add_assistant_message(&mut self, content: &str) {
        self.history.push(ChatMessage::assistant(content));
    }

    pub async fn chat(&mut self, client: &Client, stream: bool) -> anyhow::Result<Option<String>> {
        if stream {
            print!("{}: ", self.name);
            io::stdout().flush()?;
        }

        let response = send_chat(client, &self.model, &self.history, stream).await?;

        if let Some(content) = response.clone() {
            if !stream {
                println!("{}: {}", self.name, content);
            }
            self.add_assistant_message(&content);
        }

        Ok(response)
    }
}
