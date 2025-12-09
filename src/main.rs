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
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber to write to a file to avoid interfering with TUI
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("oxideagent.log")?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::sync::Arc::new(log_file))
        .init();

    let args = cli::Args::parse();
    let client = Client::new();

    let llm_config = config::LLMConfig {
        provider: "ollama".to_string(),
        api_base: args
            .llm_api_base
            .clone()
            .unwrap_or_else(config::default_api_base),
        api_key: args.llm_api_key.clone(),
        model: args.llm_model.clone(),
    };

    // Fetch the list of available Ollama models
    let available_models = match core::llm::ollama::list_models(&client, &llm_config.api_base).await
    {
        Ok(models) => models,
        Err(e) => {
            eprintln!("Error fetching Ollama models: {}", e);
            // Exit gracefully if Ollama is not available
            return Ok(());
        }
    };

    // Load configuration from file if specified, otherwise use default config
    let config_from_file = if let Some(config_path) = &args.config {
        Some(config::OxideConfig::from_file(config_path)?)
    } else {
        None
    };

    // Create base configuration from CLI arguments with defaults
    let base_config = create_default_config_from_cli(&args, &[], &llm_config);

    // Merge the configurations - CLI args take precedence over config file
    let config = merge_configs(config_from_file, base_config, &args, &[]);

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
    let mut orchestrator = container
        .build_orchestrator(orchestrator_tx, orchestrator_rx)
        .await?;

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
    let available_agents = vec![
        "Qwen".to_string(),
        "Llama".to_string(),
        "Granite".to_string(),
    ];
    let mut interface = create_interface(
        &container.config().interface,
        interface_rx,
        interface_tx,
        session_name.clone(), // Clone to keep original for logging
        session_history,
        available_agents,
        container.config().agent.model.clone(),
        available_models,
    )?;

    info!("Interface created successfully");

    // Initialize the interface
    interface.init().await?;
    info!("Interface initialized successfully");

    info!("Starting TUI interface for session: {}", session_name);

    // Run the interface
    interface.run().await?;
    info!("Interface run completed");

    info!("TUI interface ended for session: {}", session_name);

    // Cleanup the interface
    interface.cleanup().await?;
    info!("Interface cleanup completed");

    Ok(())
}

// Create a default configuration based on CLI arguments (with defaults when not specified)
fn create_default_config_from_cli(
    args: &cli::Args,
    _available_models: &[String],
    llm_config: &config::LLMConfig,
) -> config::OxideConfig {
    // Set default agent if not specified in CLI
    let agent_type = args.agent.clone().unwrap_or(cli::AgentType::Qwen);
    let model = llm_config.model.clone().unwrap_or_default();

    let agent_config = config::AgentConfig {
        agent_type: match agent_type {
            cli::AgentType::Qwen => config::AgentType::Qwen,
            cli::AgentType::Llama => config::AgentType::Llama,
            cli::AgentType::Granite => config::AgentType::Granite,
        },
        model,
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
        llm: llm_config.clone(),
    }
}

// Merge config file settings with CLI arguments (CLI takes precedence)
fn merge_configs(
    config_from_file: Option<config::OxideConfig>,
    mut base_config: config::OxideConfig,
    args: &cli::Args,
    _available_models: &[String],
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

#[allow(clippy::too_many_arguments)] // Interface creation requires all these parameters
fn create_interface(
    interface_type: &config::InterfaceType,
    rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
    session_name: String,
    session_history: Vec<ChatMessage>,
    available_agents: Vec<String>,
    current_model: String,
    available_models: Vec<String>,
) -> anyhow::Result<Box<dyn Interface>> {
    match interface_type {
        config::InterfaceType::Tui => {
            let tui = Tui::new(
                rx,
                tx,
                session_name,
                session_history,
                available_agents,
                current_model,
                available_models,
            )?;
            Ok(Box::new(tui))
        }
    }
}
