use crate::types::{ChatMessage, Tool, ToolCall};
use reqwest::Client;
use serde_json::json;

pub async fn send_chat(
    client: &Client,
    model: &str,
    history: &[ChatMessage],
    tools: &[Tool],
    stream: bool,
) -> anyhow::Result<Option<ChatMessage>> {
    let res = client
        .post("http://localhost:11434/api/chat")
        .json(&json!({
            "model": model,
            "messages": history,
            "tools": tools,
            "stream": stream,
        }))
        .send()
        .await?;

    if stream {
        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer.drain(..=newline_pos).collect::<String>();
                if line.trim().is_empty() {
                    continue;
                }

                let parsed: serde_json::Value = match serde_json::from_str(line.trim()) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("\nError parsing JSON line: '{}', error: {}", line, e);
                        continue;
                    }
                };

                // Handle content - print immediately as we receive it
                if let Some(c) = parsed["message"]["content"].as_str() {
                    // Let's add some debug output to see what's happening
                    print!("{}", c);
                    std::io::Write::flush(&mut std::io::stdout())?;
                    content.push_str(c);
                }

                // Handle tool calls
                if let Some(tool_call_array) = parsed["message"]["tool_calls"].as_array() {
                    for tool_call in tool_call_array {
                        if let Ok(tc) = serde_json::from_value::<ToolCall>(tool_call.clone()) {
                            tool_calls.push(tc);
                        }
                    }
                }

                if parsed["done"].as_bool().unwrap_or(false) {
                    println!();
                    let message = if !tool_calls.is_empty() {
                        ChatMessage::tool_call(&content, tool_calls)
                    } else {
                        ChatMessage::assistant(&content)
                    };
                    return Ok(Some(message));
                }
            }
        }
        println!();
        let message = if !tool_calls.is_empty() {
            ChatMessage::tool_call(&content, tool_calls)
        } else {
            ChatMessage::assistant(&content)
        };
        Ok(Some(message))
    } else {
        let json: serde_json::Value = res.json().await?;
        let content = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        if let Some(tool_call_array) = json["message"]["tool_calls"].as_array() {
            for tool_call in tool_call_array {
                if let Ok(tc) = serde_json::from_value::<ToolCall>(tool_call.clone()) {
                    tool_calls.push(tc);
                }
            }
        }
        
        let message = if !tool_calls.is_empty() {
            ChatMessage::tool_call(&content, tool_calls)
        } else {
            ChatMessage::assistant(&content)
        };
        Ok(Some(message))
    }
}