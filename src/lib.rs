#![allow(non_snake_case)]

//! OxideAgent library crate.
//!
//! This crate exposes the main modules for testing purposes.

pub mod cli;
pub mod config;
pub mod core;
pub mod interfaces;
pub mod types;

// Re-export main items for easier access
pub use config::Config;
