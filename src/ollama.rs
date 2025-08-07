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

        use futures_util::StreamExt;
        use std::io::{self, Write};
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let line = String::from_utf8_lossy(&chunk);
            for part in line.lines() {
                if let Some(stripped) = part.strip_prefix("data: ") {
                    let parsed: serde_json::Value = serde_json::from_str(stripped)?;
                    if let Some(c) = parsed["message"]["content"].as_str() {
                        print!("{c}");
                        io::stdout().flush()?;
                        content.push_str(c);
                    }
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
