use crate::ollama::send_chat;
use crate::types::{ChatMessage, Tool, ToolFunctionDefinition};
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};

pub struct Agent {
    pub name: String,
    pub model: String,
    pub history: Vec<ChatMessage>,
    pub tools: Vec<Tool>,
}

impl Agent {
    pub fn new(name: &str, model: &str) -> Self {
        let system_message = "You are a helpful assistant. You have access to tools that can help you perform various tasks. Use them when appropriate.";
        
        let tools = vec![
            Tool {
                r#type: "function".to_string(),
                function: ToolFunctionDefinition {
                    name: "write_file".to_string(),
                    description: "Write content to a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The path to the file to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "The content to write to the file"
                            }
                        },
                        "required": ["path", "content"]
                    }),
                },
            },
            Tool {
                r#type: "function".to_string(),
                function: ToolFunctionDefinition {
                    name: "read_file".to_string(),
                    description: "Read content from a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The path to the file to read"
                            }
                        },
                        "required": ["path"]
                    }),
                },
            },
            Tool {
                r#type: "function".to_string(),
                function: ToolFunctionDefinition {
                    name: "run_shell_command".to_string(),
                    description: "Run a shell command".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "The shell command to run"
                            }
                        },
                        "required": ["command"]
                    }),
                },
            },
        ];
        
        Self {
            name: name.to_string(),
            model: model.to_string(),
            history: vec![ChatMessage::system(system_message)],
            tools,
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.history.push(ChatMessage::user(content));
    }

    pub fn add_assistant_message(&mut self, message: ChatMessage) {
        self.history.push(message);
    }

    pub async fn chat(&mut self, client: &Client, stream: bool) -> anyhow::Result<Option<ChatMessage>> {
        if stream {
            print!("{}: ", self.name);
            io::stdout().flush()?;
        }

        let response = send_chat(client, &self.model, &self.history, &self.tools, stream).await?;

        if let Some(message) = response.clone() {
            // Don't print content here when streaming, as it's already printed in send_chat
            if !stream {
                println!("{}: {}", self.name, message.content);
            }
            self.add_assistant_message(message.clone());
        }

        Ok(response)
    }
}