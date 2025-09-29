#![allow(non_snake_case)]
#![feature(let_chains)]

mod cli;
mod config;
mod core;
mod interfaces;
mod types;

use crate::core::interface::Interface;
use crate::interfaces::tui::Tui;
use crate::types::{AppEvent, ChatMessage};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::from_args();

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
