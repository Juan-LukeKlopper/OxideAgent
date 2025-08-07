use crate::types::ChatMessage;
use reqwest::Client;
use serde_json::json;

pub async fn send_chat(
    client: &Client,
    model: &str,
    history: &[ChatMessage],
    stream: bool,
) -> anyhow::Result<Option<String>> {
    let res = client
        .post("http://localhost:11434/api/chat")
        .json(&json!({
            "model": model,
            "messages": history,
            "stream": stream,
        }))
        .send()
        .await?;

    if stream {
        let mut content = String::new();
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();

        use futures_util::StreamExt;
        use std::io::{self, Write};
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

                if let Some(c) = parsed["message"]["content"].as_str() {
                    print!("{c}");
                    io::stdout().flush()?;
                    content.push_str(c);
                }

                if parsed["done"].as_bool().unwrap_or(false) {
                    println!();
                    return Ok(Some(content));
                }
            }
        }
        println!();
        Ok(Some(content))
    } else {
        let json: serde_json::Value = res.json().await?;
        let content = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(Some(content))
    }
}
