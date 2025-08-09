mod agents;
mod cli;
mod ollama;
mod tools;
mod types;

use agents::Agent;
use clap::Parser;
use cli::Args;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashSet;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let client = Client::new();

    let mut agent = Agent::new(args.agent.name(), args.agent.model());
    let mut session_allowed_tools: HashSet<String> = HashSet::new();

    loop {
        use std::io::{self, Write};
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "/exit" {
            break;
        }

        agent.add_user_message(input);

        'agent_turn: loop {
            if let Some(response) = agent.chat(&client, !args.no_stream).await? {
                if let Some(start_index) = response.find("{\"tool\":") {
                    let json_str = &response[start_index..];
                    if let Ok(json) = json_str.parse::<Value>() {
                        let tool_name = json["tool"].as_str().unwrap_or_default().to_string();
                        let mut tool_output = None;

                        let execute = |json: &Value| -> anyhow::Result<String> {
                            match json["tool"].as_str() {
                                Some("write_file") => {
                                    let (path, content) = (
                                        json["path"].as_str().unwrap_or(""),
                                        json["content"].as_str().unwrap_or(""),
                                    );
                                    tools::write_to_file(path, content)?;
                                    Ok(format!("File '{}' written successfully.", path))
                                }
                                Some("read_file") => {
                                    let path = json["path"].as_str().unwrap_or("");
                                    tools::read_file(path)
                                }
                                Some("run_shell_command") => {
                                    let command = json["command"].as_str().unwrap_or("");
                                    tools::run_shell_command(command)
                                }
                                _ => Err(anyhow::anyhow!("Unknown tool")),
                            }
                        };

                        if !session_allowed_tools.contains(&tool_name) {
                            println!(
                                "The model wants to use the '{}' tool with the following parameters:",
                                tool_name
                            );
                            println!("{}", serde_json::to_string_pretty(&json)?);
                            println!("\nEnable? (Choose an option)");
                            println!("1. Allow once");
                            println!("2. Allow always in session");
                            println!("3. Deny");
                            println!("4. Modify");

                            let mut choice = String::new();
                            io::stdin().read_line(&mut choice)?;

                            match choice.trim() {
                                "1" => match execute(&json) {
                                    Ok(output) => tool_output = Some(output),
                                    Err(e) => tool_output = Some(format!("Error: {}", e)),
                                },
                                "2" => {
                                    session_allowed_tools.insert(tool_name.clone());
                                    match execute(&json) {
                                        Ok(output) => tool_output = Some(output),
                                        Err(e) => tool_output = Some(format!("Error: {}", e)),
                                    }
                                },
                                "3" => {
                                    tool_output = Some("Tool execution denied by user.".to_string())
                                },
                                "4" => {
                                    println!("Please enter the modified JSON for the tool call:");
                                    let mut new_json_str = String::new();
                                    io::stdin().read_line(&mut new_json_str)?;
                                    match new_json_str.parse::<Value>() {
                                        Ok(new_json) => match execute(&new_json) {
                                            Ok(output) => tool_output = Some(output),
                                            Err(e) => tool_output = Some(format!("Error: {}", e)),
                                        },
                                        Err(e) => {
                                            tool_output =
                                                Some(format!("Invalid JSON. Error: {}", e))
                                        }
                                    }
                                },
                                _ => {
                                    tool_output =
                                        Some("Invalid choice. Tool execution denied.".to_string())
                                }
                            }
                        } else {
                            match execute(&json) {
                                Ok(output) => tool_output = Some(output),
                                Err(e) => tool_output = Some(format!("Error: {}", e)),
                            }
                        }

                        if let Some(output) = tool_output {
                            println!("Tool output: {}", output);
                            agent.add_user_message(&format!(
                                "The last tool call produced this output:\n{}",
                                output
                            ));
                            // Continue the agent's turn to let it process the tool output
                            continue 'agent_turn;
                        }
                    }
                }
            }
            // If there's no tool call or the tool logic is finished, break the agent's turn
            break 'agent_turn;
        }
    }

    Ok(())
}

