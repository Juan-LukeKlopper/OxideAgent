mod agents;
mod cli;
mod ollama;
mod orchestrator;
mod tools;
mod tui;
mod types;

use agents::Agent;
use clap::Parser;
use cli::Args;
use orchestrator::Orchestrator;
use tokio::sync::mpsc;
use tools::{ReadFileTool, RunShellCommandTool, ToolRegistry, WriteFileTool};
use tui::Tui;
use types::AppEvent;

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
    let (orchestrator_tx, tui_rx) = mpsc::channel::<AppEvent>(32);
    let (tui_tx, orchestrator_rx) = mpsc::channel::<AppEvent>(32);

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

    // Get the session history to pass to the TUI
    let session_history = orchestrator.get_session_history().clone();

    // Run the orchestrator in a separate task
    tokio::spawn(async move {
        if let Err(e) = orchestrator.run().await {
            eprintln!("Orchestrator error: {}", e);
        }
    });

    // Initialize and run the TUI with the session name and history
    let mut tui = Tui::new(tui_rx, tui_tx, session_name, session_history)?;
    tui.run().await?;
    tui.restore()?;

    Ok(())
}
