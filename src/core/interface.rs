//! Interface traits for the OxideAgent system.
//!
//! This module defines the traits that all interfaces must implement to interact with the core system.

use crate::types::{AppEvent, ChatMessage};
use anyhow::Result;
use tokio::sync::mpsc;

/// Trait for handling user input from different sources
#[async_trait::async_trait]
pub trait InputHandler {
    /// Handle user input from the interface
    async fn handle_input(&mut self, input: String) -> Result<()>;
}

/// Trait for sending output to different interfaces
#[async_trait::async_trait]
pub trait OutputHandler {
    /// Send output to the interface
    async fn send_output(&mut self, output: AppEvent) -> Result<()>;
}

/// Trait for emitting events to interfaces
pub trait EventEmitter {
    /// Get the sender for sending events to the interface
    fn get_event_sender(&self) -> mpsc::Sender<AppEvent>;

    /// Get the receiver for receiving events from the interface
    fn get_event_receiver(&mut self) -> mpsc::Receiver<AppEvent>;
}

/// Trait that combines all interface traits
#[async_trait::async_trait]
pub trait Interface: InputHandler + OutputHandler + EventEmitter + Send {
    /// Initialize the interface
    async fn init(&mut self) -> Result<()>;

    /// Run the interface event loop
    async fn run(&mut self) -> Result<()>;

    /// Cleanup the interface
    async fn cleanup(&mut self) -> Result<()>;

    /// Get the session history for this interface
    fn get_session_history(&self) -> Vec<ChatMessage>;

    /// Get the session name for this interface
    fn get_session_name(&self) -> String;
}
