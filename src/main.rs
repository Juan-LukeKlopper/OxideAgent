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
    let mut orchestrator =
        Orchestrator::new(agent, tool_registry, args.no_stream, orchestrator_tx, orchestrator_rx);

    // Load the previous session state if it exists
    orchestrator.load_state()?;

    // Run the orchestrator in a separate task
    tokio::spawn(async move {
        if let Err(e) = orchestrator.run().await {
            eprintln!("Orchestrator error: {}", e);
        }
    });

    // Initialize and run the TUI
    let mut tui = Tui::new(tui_rx, tui_tx)?;
    tui.run().await?;
    tui.restore()?;

    Ok(())
}
