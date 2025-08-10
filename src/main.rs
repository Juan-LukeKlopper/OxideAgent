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
use types::ToolCall;

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
                // Check if the response contains tool calls
                if let Some(tool_calls) = &response.tool_calls {
                    if !tool_calls.is_empty() {
                        // Process each tool call with user approval
                        for tool_call in tool_calls {
                            let tool_name = tool_call.function.name.clone();
                            
                            // Check if user has already approved this tool for the session
                            let execute = if session_allowed_tools.contains(&tool_name) {
                                true
                            } else {
                                // Ask for user approval
                                println!(
                                    "The model wants to use the '{}' tool with the following parameters:",
                                    tool_name
                                );
                                println!("{}", serde_json::to_string_pretty(&tool_call.function.arguments)?);
                                println!("\nEnable? (Choose an option)");
                                println!("1. Allow once");
                                println!("2. Allow always in session");
                                println!("3. Deny");

                                let mut choice = String::new();
                                io::stdin().read_line(&mut choice)?;

                                match choice.trim() {
                                    "1" => true,
                                    "2" => {
                                        session_allowed_tools.insert(tool_name.clone());
                                        true
                                    }
                                    "3" => false,
                                    _ => {
                                        println!("Invalid choice. Tool execution denied.");
                                        false
                                    }
                                }
                            };

                            if execute {
                                let tool_output = execute_tool(&tool_call).await?;
                                println!("Tool output: {}", tool_output);
                                
                                // Add the tool output to the conversation
                                agent.add_user_message(&format!(
                                    "The tool '{}' produced this output:\n{}",
                                    tool_call.function.name, tool_output
                                ));
                            } else {
                                let denial_message = "Tool execution denied by user.";
                                println!("Tool output: {}", denial_message);
                                agent.add_user_message(&format!(
                                    "The tool '{}' was not executed:\n{}",
                                    tool_call.function.name, denial_message
                                ));
                            }
                        }
                        
                        // Continue the agent's turn to let it process the tool output
                        continue 'agent_turn;
                    }
                }
            }
            // If there's no tool call or the tool logic is finished, break the agent's turn
            break 'agent_turn;
        }
    }

    Ok(())
}

async fn execute_tool(tool_call: &ToolCall) -> anyhow::Result<String> {
    match tool_call.function.name.as_str() {
        "write_file" => {
            let args = &tool_call.function.arguments;
            let path = args["path"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            tools::write_to_file(path, content)?;
            Ok(format!("File '{}' written successfully.", path))
        }
        "read_file" => {
            let args = &tool_call.function.arguments;
            let path = args["path"].as_str().unwrap_or("");
            tools::read_file(path)
        }
        "run_shell_command" => {
            let args = &tool_call.function.arguments;
            let command = args["command"].as_str().unwrap_or("");
            tools::run_shell_command(command)
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_call.function.name)),
    }
}