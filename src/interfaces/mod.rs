//! Interface implementations for the OxideAgent system.
//!
//! This module contains implementations of the interface traits defined in `core::interface`
//! for different types of interfaces (TUI, Web, Telegram, etc.)

pub mod tui;

pub mod adapters;
pub mod capabilities;
pub mod web;
