mod agents;
mod cli;
mod ollama;
mod orchestrator;
mod tools;
mod types;

use agents::Agent;
use clap::Parser;
use cli::Args;
use orchestrator::Orchestrator;
use tools::{ReadFileTool, RunShellCommandTool, ToolRegistry, WriteFileTool};

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

    // Create the orchestrator
    let mut orchestrator = Orchestrator::new(agent, tool_registry, args.no_stream);

    // Load the previous session state if it exists
    orchestrator.load_state()?;

    // Run the main application loop
    orchestrator.run().await?;

    Ok(())
}
