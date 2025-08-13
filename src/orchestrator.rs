use crate::agents::Agent;
use crate::tools::ToolRegistry;
use crate::types::{ChatMessage, ToolCall};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const SESSION_FILE: &str = "session.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionState {
    history: Vec<ChatMessage>,
}

impl SessionState {
    fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }
}

pub struct Orchestrator {
    agent: Agent,
    tool_registry: ToolRegistry,
    client: Client,
    session_state: SessionState,
    no_stream: bool,
}

impl Orchestrator {
    pub fn new(agent: Agent, tool_registry: ToolRegistry, no_stream: bool) -> Self {
        Self {
            agent,
            tool_registry,
            client: Client::new(),
            session_state: SessionState::new(),
            no_stream,
        }
    }

    pub fn load_state(&mut self) -> anyhow::Result<()> {
        if Path::new(SESSION_FILE).exists() {
            let session_json = fs::read_to_string(SESSION_FILE)?;
            if session_json.trim().is_empty() {
                println!("Starting new session.");
                self.session_state = SessionState::new();
            } else {
                let session_state: SessionState = serde_json::from_str(&session_json)?;
                self.agent.history = session_state.history.clone();
                self.session_state = session_state;
                println!("Welcome back! Resuming previous session.");
            }
        } else {
            println!("Starting new session.");
        }
        Ok(())
    }

    fn save_state(&mut self) -> anyhow::Result<()> {
        self.session_state.history = self.agent.history.clone();
        let session_json = serde_json::to_string_pretty(&self.session_state)?;
        fs::write(SESSION_FILE, session_json)?;
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut session_allowed_tools: HashSet<String> = HashSet::new();

        loop {
            print!("You: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input == "/exit" {
                break;
            }

            self.agent.add_user_message(input);

            'agent_turn: loop {
                let tool_definitions = self.tool_registry.definitions();
                if let Some(response) = self
                    .agent
                    .chat(&self.client, &tool_definitions, !self.no_stream)
                    .await?
                {
                    if let Some(tool_calls) = &response.tool_calls {
                        if !tool_calls.is_empty() {
                            println!("\nThe model wants to use {} tool(s):", tool_calls.len());
                            for (i, tool_call) in tool_calls.iter().enumerate() {
                                let tool_name = tool_call.function.name.clone();
                                println!("\n{}. Tool: {}", i + 1, tool_name);
                                println!(
                                    "   Parameters: {}",
                                    serde_json::to_string_pretty(&tool_call.function.arguments)?
                                );
                            }

                            let mut approved_tools = Vec::new();
                            if tool_calls.len() > 5 {
                                // Handle multiple tool calls approval
                            }

                            if approved_tools.is_empty() {
                                for (i, tool_call) in tool_calls.iter().enumerate() {
                                    let tool_name = tool_call.function.name.clone();
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
                                        "4" => break,
                                        "5" => {
                                            approved_tools.push((i, true));
                                            for j in (i + 1)..tool_calls.len() {
                                                approved_tools.push((j, true));
                                            }
                                            break;
                                        }
                                        _ => approved_tools.push((i, false)),
                                    }
                                }
                            }

                            let mut any_executed = false;
                            for (index, approved) in approved_tools {
                                let tool_call = &tool_calls[index];
                                if approved {
                                    any_executed = true;
                                    let tool_output = 
                                        self.execute_tool(&tool_call).await?;
                                    println!("Tool '{}' output: {}", tool_call.function.name, tool_output);
                                    self.agent.add_user_message(&format!(
                                        "The tool '{}' produced this output:\n{}",
                                        tool_call.function.name, tool_output
                                    ));
                                } else {
                                    println!("Tool '{}' execution denied by user.", tool_call.function.name);
                                    self.agent.add_user_message(&format!(
                                        "The tool '{}' was not executed:\n{}",
                                        tool_call.function.name, "Tool execution denied by user."
                                    ));
                                }
                            }

                            if any_executed {
                                self.save_state()?;
                                continue 'agent_turn;
                            }
                        }
                    }
                }
                self.save_state()?;
                break 'agent_turn;
            }
        }
        Ok(())
    }

    async fn execute_tool(&self, tool_call: &ToolCall) -> anyhow::Result<String> {
        if let Some(tool) = self.tool_registry.get_tool(&tool_call.function.name) {
            tool.execute(&tool_call.function.arguments)
        } else {
            Err(anyhow::anyhow!(
                "Unknown tool: {}",
                tool_call.function.name
            ))
        }
    }
}