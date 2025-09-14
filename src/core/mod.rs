//! Core services for the OxideAgent system.
//!
//! This module contains the core business logic of the application, separated from
//! interface implementations to enable multiple interface types (TUI, Web, Telegram, etc.)

pub mod agents;
pub mod container;
pub mod events;
pub mod interface;
pub mod llm;
pub mod orchestrator;
pub mod session;
pub mod tools;
