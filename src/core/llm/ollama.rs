use crate::{
    core::llm::client::LlmClient,
    types::{AppEvent, ChatMessage, Tool, ToolCall},
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

pub async fn list_models(client: &Client, api_base: &str) -> anyhow::Result<Vec<String>> {
    let url = format!("{}/api/tags", api_base);
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to list models: {}",
            response.status()
        ));
    }

    let json: serde_json::Value = response.json().await?;
    let mut models = Vec::new();

    if let Some(models_array) = json["models"].as_array() {
        for model in models_array {
            if let Some(name) = model["name"].as_str() {
                models.push(name.to_string());
            }
        }
    }

    Ok(models)
}

#[derive(Debug, Clone)]
pub struct OllamaClient {
    pub client: Client,
    pub api_base: String,
}

impl OllamaClient {
    pub fn new(api_base: &str) -> Self {
        let client = Client::builder().no_proxy().build().unwrap_or_else(|error| {
            warn!(
                "Failed to build reqwest client with no_proxy, falling back to default client: {}",
                error
            );
            Client::new()
        });

        Self {
            client,
            api_base: api_base.to_string(),
        }
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn chat(
        &self,
        model: &str,
        history: &[ChatMessage],
        tools: &[Tool],
        stream: bool,
        tx: mpsc::Sender<AppEvent>,
    ) -> anyhow::Result<Option<ChatMessage>> {
        info!("=== OLLAMA REQUEST START ===");
        info!("Sending request to Ollama at {}", self.api_base);
        info!("Model: {}", model);
        info!("History length: {} messages", history.len());
        info!("Streaming: {}", stream);

        let url = format!("{}/api/chat", self.api_base);

        let mut request_body = json!({
            "model": model,
            "messages": history,
            "stream": stream,
        });

        if !tools.is_empty() {
            info!("Adding {} tools to request", tools.len());
            request_body["tools"] = json!(tools);
        }

        let response_result = self.client.post(&url).json(&request_body).send().await;

        info!("=== OLLAMA REQUEST END ===");

        match response_result {
            Ok(response) => {
                info!("=== OLLAMA RESPONSE START ===");
                let status = response.status();
                info!("Received HTTP response with status: {}", status);

                if stream {
                    info!("Processing streaming response...");
                    let mut content = String::new();
                    let mut tool_calls: Vec<ToolCall> = Vec::new();
                    let mut stream = response.bytes_stream();
                    let mut buffer = String::new();

                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(chunk_data) => {
                                trace!("Received {} bytes from Ollama stream", chunk_data.len());
                                buffer.push_str(&String::from_utf8_lossy(&chunk_data));

                                while let Some(newline_pos) = buffer.find('\n') {
                                    let line = buffer.drain(..=newline_pos).collect::<String>();
                                    if line.trim().is_empty() {
                                        continue;
                                    }

                                    debug!("Received streaming line: {}", line.trim());

                                    let parsed: serde_json::Value =
                                        match serde_json::from_str(line.trim()) {
                                            Ok(p) => p,
                                            Err(e) => {
                                                error!(
                                                    "Error parsing JSON line: '{}', error: {}",
                                                    line, e
                                                );
                                                continue;
                                            }
                                        };

                                    if let Some(c) = parsed["message"]["content"].as_str() {
                                        debug!("Content chunk: {}", c);
                                        // Send content chunk to UI
                                        if tx
                                            .send(AppEvent::AgentStreamChunk(c.to_string()))
                                            .await
                                            .is_err()
                                        {
                                            error!("Failed to send stream chunk to UI");
                                            break;
                                        }
                                        content.push_str(c);
                                    }

                                    if let Some(tool_call_array) =
                                        parsed["message"]["tool_calls"].as_array()
                                    {
                                        debug!(
                                            "Found {} tool calls in streaming response",
                                            tool_call_array.len()
                                        );
                                        for tool_call in tool_call_array {
                                            if let Ok(tc) = serde_json::from_value::<ToolCall>(
                                                tool_call.clone(),
                                            ) {
                                                debug!(
                                                    "Tool call: {} with args: {}",
                                                    tc.function.name, tc.function.arguments
                                                );
                                                tool_calls.push(tc);
                                            }
                                        }
                                    }

                                    if parsed["done"].as_bool().unwrap_or(false) {
                                        info!("Streaming response completed");
                                        if tx.send(AppEvent::AgentStreamEnd).await.is_err() {
                                            error!("Failed to send stream end to UI");
                                        }
                                        let message = if !tool_calls.is_empty() {
                                            info!(
                                                "Response contains {} tool calls",
                                                tool_calls.len()
                                            );
                                            ChatMessage::tool_call(&content, tool_calls)
                                        } else {
                                            ChatMessage::assistant(&content)
                                        };
                                        info!("=== OLLAMA RESPONSE END ===");
                                        return Ok(Some(message));
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading stream chunk: {}", e);
                                break;
                            }
                        }
                    }

                    if tx.send(AppEvent::AgentStreamEnd).await.is_err() {
                        error!("Failed to send stream end to UI");
                    }

                    let message = if !tool_calls.is_empty() {
                        info!("Final response contains {} tool calls", tool_calls.len());
                        ChatMessage::tool_call(&content, tool_calls)
                    } else {
                        ChatMessage::assistant(&content)
                    };
                    info!("=== OLLAMA RESPONSE END ===");
                    Ok(Some(message))
                } else {
                    info!("Processing non-streaming response...");
                    // Non-streaming case
                    let json: serde_json::Value = match response.json().await {
                        Ok(j) => {
                            info!("Successfully parsed JSON response");
                            j
                        }
                        Err(e) => {
                            error!("Error parsing JSON response: {}", e);
                            return Err(e.into());
                        }
                    };

                    debug!(
                        "Full JSON response: {}",
                        serde_json::to_string_pretty(&json)?
                    );

                    let content = json["message"]["content"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    info!("Response content length: {} characters", content.len());

                    let mut tool_calls: Vec<ToolCall> = Vec::new();
                    if let Some(tool_call_array) = json["message"]["tool_calls"].as_array() {
                        info!("Found {} tool calls in response", tool_call_array.len());
                        for (i, tool_call) in tool_call_array.iter().enumerate() {
                            if let Ok(tc) = serde_json::from_value::<ToolCall>(tool_call.clone()) {
                                info!(
                                    "  {}. Tool call: {} with args: {}",
                                    i + 1,
                                    tc.function.name,
                                    tc.function.arguments
                                );
                                tool_calls.push(tc);
                            }
                        }
                    } else {
                        info!("No tool calls found in response");
                    }

                    let message = if !tool_calls.is_empty() {
                        info!(
                            "Creating tool call message with {} tool calls",
                            tool_calls.len()
                        );
                        ChatMessage::tool_call(&content, tool_calls)
                    } else {
                        info!("Creating assistant message");
                        ChatMessage::assistant(&content)
                    };

                    info!("=== OLLAMA RESPONSE END ===");
                    Ok(Some(message))
                }
            }
            Err(e) => {
                error!("=== OLLAMA REQUEST FAILED ===");
                error!("Failed to send request to Ollama: {}", e);
                error!("=== OLLAMA REQUEST FAILED END ===");
                Err(e.into())
            }
        }
    }
}
