// Simple test to verify tool sending to Ollama

use serde_json::json;

fn main() {
    println!("Testing tool payload structure...");

    // Example tool structure that would be sent to Ollama
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
                "description": "A detailed tool for dynamic and reflective problem-solving through thoughts...",
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

    let request_payload = json!({
        "model": "qwen3:4b",
        "messages": [
            {"role": "user", "content": "Can you help me solve this problem?"}
        ],
        "tools": tools,
        "stream": false
    });

    println!("Full request payload that would be sent to Ollama:");
    println!(
        "{}",
        serde_json::to_string_pretty(&request_payload).unwrap()
    );
}
