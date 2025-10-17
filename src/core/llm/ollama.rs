use crate::types::{AppEvent, ChatMessage, Tool, ToolCall};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace};

#[derive(Debug, Serialize, Deserialize)]
struct OllamaTag {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaTags {
    models: Vec<OllamaTag>,
}

pub async fn list_models(client: &Client, api_base: &str) -> anyhow::Result<Vec<String>> {
    let url = format!("{}/api/tags", api_base);
    let response = client.get(&url).send().await?;
    let tags: OllamaTags = response.json().await?;
    Ok(tags.models.into_iter().map(|t| t.name).collect())
}

pub async fn send_chat(
    client: &Client,
    model: &str,
    history: &[ChatMessage],
    tools: &[Tool],
    stream: bool,
    tx: mpsc::Sender<AppEvent>,
    api_base: &str,
) -> anyhow::Result<Option<ChatMessage>> {
    info!("=== OLLAMA REQUEST START ===");
    info!("Sending chat request to Ollama model: {}", model);
    info!("Request streaming: {}", stream);
    info!("Message history contains {} messages", history.len());
    info!("Sending {} tools to model:", tools.len());

    for (i, tool) in tools.iter().enumerate() {
        let description = if tool.function.description.len() > 60 {
            format!("{}...", &tool.function.description[..60])
        } else {
            tool.function.description.clone()
        };
        info!(
            "  {}. Tool: {} - {}",
            i + 1,
            tool.function.name,
            description
        );
    }

    // Create the request payload
    let request_payload = json!({
        "model": model,
        "messages": history,
        "tools": tools,
        "stream": stream,
    });

    // Log the full request payload (truncated if too long)
    let payload_str = serde_json::to_string_pretty(&request_payload)?;
    if payload_str.len() > 5000 {
        info!("Full request payload (truncated): {}", &payload_str[..5000]);
        info!(
            "... (payload truncated, total length: {} characters)",
            payload_str.len()
        );
    } else {
        info!("Full request payload:\n{}", payload_str);
    }

    let url = format!("{}/api/chat", api_base);
    info!("Making HTTP POST request to: {}", url);

    // Add detailed HTTP request tracing
    trace!("Sending HTTP request with headers and payload to Ollama API");
    let response_result = client.post(&url).json(&request_payload).send().await;

    match &response_result {
        Ok(response) => {
            trace!(
                "HTTP request to Ollama succeeded with status: {}",
                response.status()
            );
        }
        Err(e) => {
            trace!("HTTP request to Ollama failed with error: {}", e);
        }
    }

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
                                        if let Ok(tc) =
                                            serde_json::from_value::<ToolCall>(tool_call.clone())
                                        {
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
                                        info!("Response contains {} tool calls", tool_calls.len());
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
