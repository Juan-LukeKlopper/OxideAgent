#![allow(non_snake_case)]

mod cli;
mod config;
mod core;
mod interfaces;
mod types;

use crate::core::interface::Interface;
use crate::interfaces::tui::Tui;
use crate::types::{AppEvent, ChatMessage};
use clap::Parser;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    // Load configuration from file if specified, otherwise use default config
    let config_from_file = if let Some(config_path) = &args.config {
        Some(config::OxideConfig::from_file(config_path)?)
    } else {
        None
    };

    // Create base configuration from CLI arguments with defaults
    let base_config = create_default_config_from_cli(&args);

    // Merge the configurations - CLI args take precedence over config file
    let config = merge_configs(config_from_file, base_config, &args);

    // Validate the configuration
    config.validate()?;

    // Handle session listing if requested
    if config.list_sessions {
        match crate::core::session::SessionManager::list_sessions() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    println!("No sessions found.");
                } else {
                    println!("Available sessions:");
                    for session in sessions {
                        println!("  - {}", session);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error listing sessions: {}", e);
            }
        }
        return Ok(());
    }

    // Create the container
    let mut container = crate::core::container::Container::new(config);

    // Determine the session name for display
    let session_name = container
        .config()
        .session
        .clone()
        .unwrap_or_else(|| "default".to_string());

    // Create channels for communication
    let (orchestrator_tx, interface_rx) = mpsc::channel::<AppEvent>(32);
    let (interface_tx, orchestrator_rx) = mpsc::channel::<AppEvent>(32);

    // Build the orchestrator using the container
    let mut orchestrator = container.build_orchestrator(orchestrator_tx, orchestrator_rx)?;

    // Load the previous session state if it exists
    orchestrator.load_state()?;

    // Get the session history to pass to the interface
    let session_history = orchestrator.get_session_history().clone();

    // Run the orchestrator in a separate task
    tokio::spawn(async move {
        if let Err(e) = orchestrator.run().await {
            eprintln!("Orchestrator error: {}", e);
        }
    });

    // Create the interface (TUI in this case)
    let mut interface = create_interface(
        &container.config().interface,
        interface_rx,
        interface_tx,
        session_name,
        session_history,
    )?;

    // Initialize the interface
    interface.init().await?;

    // Run the interface
    interface.run().await?;

    // Cleanup the interface
    interface.cleanup().await?;

    Ok(())
}

// Create a default configuration based on CLI arguments (with defaults when not specified)
fn create_default_config_from_cli(args: &cli::Args) -> config::OxideConfig {
    // Set default agent if not specified in CLI
    let agent_type = args.agent.clone().unwrap_or(cli::AgentType::Qwen);
    let agent_config = config::AgentConfig {
        agent_type: match agent_type {
            cli::AgentType::Qwen => config::AgentType::Qwen,
            cli::AgentType::Llama => config::AgentType::Llama,
            cli::AgentType::Granite => config::AgentType::Granite,
        },
        model: match agent_type {
            cli::AgentType::Qwen => "qwen3:4b".to_string(),
            cli::AgentType::Llama => "llama3.2".to_string(),
            cli::AgentType::Granite => "smolLM2".to_string(),
        },
        name: agent_type.name().to_string(),
        system_prompt: agent_type.system_prompt().to_string(),
    };

    config::OxideConfig {
        agent: agent_config,
        no_stream: args.no_stream.unwrap_or(false),
        session: args.session.clone(),
        list_sessions: args.list_sessions.unwrap_or(false),
        interface: args
            .interface
            .clone()
            .unwrap_or(cli::InterfaceType::Tui)
            .into(),
        mcp: config::MCPConfig {
            server: args.mcp_server.clone(),
            auth_token: args.mcp_auth_token.clone(),
            tools: vec![],
        },
        llm: config::LLMConfig {
            provider: "ollama".to_string(),
            api_base: None,
            api_key: None,
            model: None,
        },
    }
}

// Merge config file settings with CLI arguments (CLI takes precedence)
fn merge_configs(
    config_from_file: Option<config::OxideConfig>,
    mut base_config: config::OxideConfig,
    args: &cli::Args,
) -> config::OxideConfig {
    match config_from_file {
        Some(file_config) => {
            // Override file config with CLI args if provided
            if args.agent.is_some() {
                // Use CLI values for agent
                let agent_type = args.agent.clone().unwrap();
                base_config.agent.agent_type = match agent_type {
                    cli::AgentType::Qwen => config::AgentType::Qwen,
                    cli::AgentType::Llama => config::AgentType::Llama,
                    cli::AgentType::Granite => config::AgentType::Granite,
                };
                base_config.agent.model = match agent_type {
                    cli::AgentType::Qwen => "qwen3:4b".to_string(),
                    cli::AgentType::Llama => "llama3.2".to_string(),
                    cli::AgentType::Granite => "smolLM2".to_string(),
                };
                base_config.agent.name = agent_type.name().to_string();
                base_config.agent.system_prompt = agent_type.system_prompt().to_string();
            } else {
                // Use file config values for agent
                base_config.agent = file_config.agent;
            }

            if args.no_stream.is_some() {
                base_config.no_stream = args.no_stream.unwrap();
            } else {
                base_config.no_stream = file_config.no_stream;
            }

            if args.session.is_some() {
                base_config.session = args.session.clone();
            } else {
                base_config.session = file_config.session;
            }

            if args.list_sessions.is_some() {
                base_config.list_sessions = args.list_sessions.unwrap();
            } else {
                base_config.list_sessions = file_config.list_sessions;
            }

            if args.interface.is_some() {
                base_config.interface = args.interface.clone().unwrap().into();
            } else {
                base_config.interface = file_config.interface;
            }

            // For MCP, CLI args take precedence but we also keep file config values
            if args.mcp_server.is_some() {
                base_config.mcp.server = args.mcp_server.clone();
            } else {
                base_config.mcp.server = file_config.mcp.server;
            }

            if args.mcp_auth_token.is_some() {
                base_config.mcp.auth_token = args.mcp_auth_token.clone();
            } else {
                base_config.mcp.auth_token = file_config.mcp.auth_token;
            }

            base_config.mcp.tools = file_config.mcp.tools; // Keep file config tools

            // For LLM config, use file config but allow CLI to influence it
            base_config.llm = file_config.llm;

            base_config
        }
        None => base_config, // Use CLI defaults only
    }
}

fn create_interface(
    interface_type: &config::InterfaceType,
    rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
    session_name: String,
    session_history: Vec<ChatMessage>,
) -> anyhow::Result<Box<dyn Interface>> {
    match interface_type {
        config::InterfaceType::Tui => {
            let tui = Tui::new(rx, tx, session_name, session_history)?;
            Ok(Box::new(tui))
        }
    }
}
