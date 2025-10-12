// Test to verify that tools are properly sent to Ollama

use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create the tools payload exactly as the agent would
    let tools = vec![
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file",
                "parameters": {
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
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read content from a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "run_shell_command",
                "description": "Run a shell command",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to run"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "sequentialthinking",
                "description": "A detailed tool for dynamic and reflective problem-solving through thoughts. This tool helps analyze problems through a flexible thinking process that can adapt and evolve. Each thought can build on, question, or revise previous insights as understanding deepens.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "thought": {
                            "type": "string",
                            "description": "Your current thinking step"
                        },
                        "nextThoughtNeeded": {
                            "type": "boolean",
                            "description": "Whether another thought step is needed"
                        },
                        "thoughtNumber": {
                            "type": "integer",
                            "description": "Current thought number",
                            "minimum": 1
                        },
                        "totalThoughts": {
                            "type": "integer",
                            "description": "Estimated total thoughts needed",
                            "minimum": 1
                        },
                        "isRevision": {
                            "type": "boolean",
                            "description": "Whether this revises previous thinking"
                        },
                        "revisesThought": {
                            "type": "integer",
                            "description": "Which thought is being reconsidered",
                            "minimum": 1
                        },
                        "branchFromThought": {
                            "type": "integer",
                            "description": "Branching point thought number",
                            "minimum": 1
                        },
                        "branchId": {
                            "type": "string",
                            "description": "Branch identifier"
                        },
                        "needsMoreThoughts": {
                            "type": "boolean",
                            "description": "If more thoughts are needed"
                        }
                    },
                    "required": ["thought", "nextThoughtNeeded", "thoughtNumber", "totalThoughts"]
                }
            }
        }),
    ];

    // Create a simple message history
    let messages = vec![json!({
        "role": "user",
        "content": "Can you help me solve this problem using the available tools?"
    })];

    // Create the request payload exactly as the agent would
    let request_payload = json!({
        "model": "qwen3:4b",
        "messages": messages,
        "tools": tools,
        "stream": false
    });

    println!("Full request payload that would be sent to Ollama:");
    println!("{}", serde_json::to_string_pretty(&request_payload)?);

    // Create HTTP client
    let client = Client::new();

    // Send the request (this will likely fail since we're not making a real tool call,
    // but we can see the payload)
    println!("\nSending request to Ollama...");

    match client
        .post("http://localhost:11434/api/chat")
        .json(&request_payload)
        .send()
        .await
    {
        Ok(response) => {
            println!("Request sent successfully!");
            println!("Status: {}", response.status());

            // Try to get the response text
            match response.text().await {
                Ok(text) => {
                    println!("Response body:");
                    println!("{}", text);
                }
                Err(e) => {
                    println!("Error reading response body: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Error sending request: {}", e);
        }
    }

    Ok(())
}
