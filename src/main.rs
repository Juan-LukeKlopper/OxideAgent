mod cli;
mod core;
mod interfaces;
mod types;

use crate::core::agents::Agent;
use crate::core::interface::Interface;
use crate::core::orchestrator::Orchestrator;
use crate::core::tools::{ReadFileTool, RunShellCommandTool, ToolRegistry, WriteFileTool};
use crate::interfaces::tui::Tui;
use crate::types::{AppEvent, ChatMessage};
use clap::Parser;
use cli::Args;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Handle session listing if requested
    if args.list_sessions {
        match Orchestrator::list_sessions() {
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

    // Determine the session name for display
    let session_name = args.session.clone().unwrap_or_else(|| "default".to_string());

    // Create the agent
    let agent = Agent::new(args.agent.name(), args.agent.model());

    // Create and populate the tool registry
    let mut tool_registry = ToolRegistry::new();
    tool_registry.add_tool(Box::new(WriteFileTool));
    tool_registry.add_tool(Box::new(ReadFileTool));
    tool_registry.add_tool(Box::new(RunShellCommandTool));

    // Create channels for communication
    let (orchestrator_tx, interface_rx) = mpsc::channel::<AppEvent>(32);
    let (interface_tx, orchestrator_rx) = mpsc::channel::<AppEvent>(32);

    // Create the orchestrator
    let mut orchestrator = Orchestrator::new(
        agent,
        tool_registry,
        args.session,
        args.no_stream,
        orchestrator_tx,
        orchestrator_rx,
    );

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
    let mut interface = create_interface(args.interface, interface_rx, interface_tx, session_name, session_history)?;
    
    // Initialize the interface
    interface.init().await?;
    
    // Run the interface
    interface.run().await?;
    
    // Cleanup the interface
    interface.cleanup().await?;

    Ok(())
}

fn create_interface(
    interface_type: cli::InterfaceType,
    rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
    session_name: String,
    session_history: Vec<ChatMessage>,
) -> anyhow::Result<Box<dyn Interface>> {
    match interface_type {
        cli::InterfaceType::Tui => {
            let tui = Tui::new(rx, tx, session_name, session_history)?;
            Ok(Box::new(tui))
        }
    }
}