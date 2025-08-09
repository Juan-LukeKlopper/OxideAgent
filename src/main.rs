mod agents;
mod cli;
mod ollama;
mod tools;
mod types;

use agents::Agent;
use clap::Parser;
use cli::Args;
use regex::Regex;
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
        if let Some(response) = agent.chat(&client, !args.no_stream).await? {
            // A simple find is more robust than a regex for this case.
            if let Some(start_index) = response.find("{\"tool\":") {
                let json_str = &response[start_index..];
                if let Ok(json) = json_str.parse::<Value>() {
                    let tool_name = json["tool"].as_str().unwrap_or_default().to_string();

                    let execute_tool = |json: &Value| {
                        if let (Some(path), Some(content)) = 
                            (json["path"].as_str(), json["content"].as_str())
                        {
                            match tools::write_to_file(path, content) {
                                Ok(_) => println!("Wrote file '{}'", path), 
                                Err(e) => eprintln!("Error writing file: {}", e),
                            }
                        }
                    };

                    if session_allowed_tools.contains(&tool_name) {
                        execute_tool(&json);
                        continue;
                    }

                    println!(
                        "The model wants to use the '{}' tool with the following parameters:",
                        tool_name
                    );
                    println!("{}", serde_json::to_string_pretty(&json)?);
                    println!("\nEnable? (Choose an option)");
                    println!("1. Allow once");
                    println!("2. Allow always in session");
                    println!("3. Allow always (not implemented)");
                    println!("4. Deny");
                    println!("5. Modify");

                    let mut choice = String::new();
                    io::stdin().read_line(&mut choice)?;

                    match choice.trim() {
                        "1" => execute_tool(&json),
                        "2" => {
                            session_allowed_tools.insert(tool_name);
                            execute_tool(&json);
                        }
                        "3" => {
                            println!("'Allow always' is not implemented yet. Allowing once.");
                            execute_tool(&json);
                        }
                        "4" => println!("Tool execution denied."),
                        "5" => {
                            println!("Please enter the modified JSON for the tool call:");
                            let mut new_json_str = String::new();
                            io::stdin().read_line(&mut new_json_str)?;
                            match new_json_str.parse::<Value>() {
                                Ok(new_json) => {
                                    println!("Executing with modified parameters.");
                                    execute_tool(&new_json);
                                }
                                Err(e) => {
                                    eprintln!("Invalid JSON. Tool execution denied. Error: {}", e)
                                }
                            }
                        }
                        _ => println!("Invalid choice. Denying tool execution."),
                    }
                }
            }
        }
    }

    Ok(())
}
