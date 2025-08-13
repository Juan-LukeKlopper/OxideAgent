mod agents;
mod cli;
mod ollama;
mod tools;
mod types;

use agents::Agent;
use clap::Parser;
use cli::Args;
use reqwest::Client;
use std::collections::HashSet;
use tokio;
use tools::{ReadFileTool, RunShellCommandTool, ToolRegistry, WriteFileTool};
use types::ToolCall;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let client = Client::new();

    // Create and populate the tool registry
    let mut tool_registry = ToolRegistry::new();
    tool_registry.add_tool(Box::new(WriteFileTool));
    tool_registry.add_tool(Box::new(ReadFileTool));
    tool_registry.add_tool(Box::new(RunShellCommandTool));

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
            let tool_definitions = tool_registry.definitions();
            if let Some(response) = agent
                .chat(&client, &tool_definitions, !args.no_stream)
                .await? {
                // Check if the response contains tool calls
                if let Some(tool_calls) = &response.tool_calls {
                    if !tool_calls.is_empty() {
                        // Show all tool calls at once
                        println!("\nThe model wants to use {} tool(s):", tool_calls.len());
                        for (i, tool_call) in tool_calls.iter().enumerate() {
                            let tool_name = tool_call.function.name.clone();
                            println!("\n{}. Tool: {}", i + 1, tool_name);
                            println!("   Parameters: {}", serde_json::to_string_pretty(&tool_call.function.arguments)?);
                        }
                        
                        // If there are many tool calls, give option to approve all or deny all
                        let mut approved_tools = Vec::new();
                        
                        if tool_calls.len() > 5 {
                            println!("\nThere are many tool calls ({}). What would you like to do?", tool_calls.len());
                            println!("1. Approve all");
                            println!("2. Deny all");
                            println!("3. Review individually");
                            println!("4. Deny and continue");
                            
                            let mut choice = String::new();
                            io::stdin().read_line(&mut choice)?;
                            
                            match choice.trim() {
                                "1" => {
                                    // Approve all
                                    for (i, _tool_call) in tool_calls.iter().enumerate() {
                                        approved_tools.push((i, true));
                                    }
                                }
                                "2" | "4" => {
                                    // Deny all or deny and continue
                                    let denial_message = if choice.trim() == "2" {
                                        "All tool executions denied by user."
                                    } else {
                                        "Tool execution denied by user."
                                    };
                                    println!("{}", denial_message);
                                    agent.add_user_message(denial_message);
                                    if choice.trim() == "2" {
                                        break 'agent_turn;
                                    }
                                    continue 'agent_turn;
                                }
                                "3" => {
                                    // Review individually - fall through to individual review
                                }
                                _ => {
                                    println!("Invalid choice. Denying all tool executions.");
                                    break 'agent_turn;
                                }
                            }
                        }
                        
                        // If we haven't approved all, review individually
                        if approved_tools.is_empty() {
                            for (i, tool_call) in tool_calls.iter().enumerate() {
                                let tool_name = tool_call.function.name.clone();
                                
                                // Check if user has already approved this tool for the session
                                if session_allowed_tools.contains(&tool_name) {
                                    approved_tools.push((i, true));
                                    continue;
                                }
                                
                                println!("\nTool {}: {}", i + 1, tool_name);
                                println!("Parameters: {}", serde_json::to_string_pretty(&tool_call.function.arguments)?);
                                println!("Enable? (1=Allow once, 2=Allow always, 3=Deny, 4=Deny all and continue, 5=Approve all remaining)");
                                
                                let mut choice = String::new();
                                io::stdin().read_line(&mut choice)?;
                                
                                match choice.trim() {
                                    "1" => approved_tools.push((i, true)),
                                    "2" => {
                                        session_allowed_tools.insert(tool_name.clone());
                                        approved_tools.push((i, true));
                                    }
                                    "3" => approved_tools.push((i, false)),
                                    "4" => {
                                        // Deny this and all remaining, but continue
                                        println!("Tool execution denied by user.");
                                        break;
                                    }
                                    "5" => {
                                        // Approve this and all remaining
                                        approved_tools.push((i, true));
                                        for j in (i + 1)..tool_calls.len() {
                                            approved_tools.push((j, true));
                                        }
                                        break;
                                    }
                                    _ => {
                                        println!("Invalid choice. Denying this tool.");
                                        approved_tools.push((i, false));
                                    }
                                }
                            }
                        }
                        
                        // Execute approved tools
                        let mut any_executed = false;
                        for (index, approved) in approved_tools {
                            let tool_call = &tool_calls[index];
                            if approved {
                                any_executed = true;
                                let tool_output =
                                    execute_tool(&tool_registry, &tool_call).await?;
                                println!("Tool '{}' output: {}", tool_call.function.name, tool_output);
                                
                                // Add the tool output to the conversation
                                agent.add_user_message(&format!(
                                    "The tool '{}' produced this output:\n{}",
                                    tool_call.function.name, tool_output
                                ));
                            } else {
                                println!("Tool '{}' execution denied by user.", tool_call.function.name);
                                agent.add_user_message(&format!(
                                    "The tool '{}' was not executed:\n{}",
                                    tool_call.function.name, "Tool execution denied by user."
                                ));
                            }
                        }
                        
                        // If any tools were executed, continue the agent's turn to process the output
                        if any_executed {
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

async fn execute_tool(
    registry: &ToolRegistry,
    tool_call: &ToolCall,
) -> anyhow::Result<String> {
    if let Some(tool) = registry.get_tool(&tool_call.function.name) {
        tool.execute(&tool_call.function.arguments)
    } else {
        Err(anyhow::anyhow!(
            "Unknown tool: {}",
            tool_call.function.name
        ))
    }
}