# OxideAgent Architecture

This document describes the architecture of the OxideAgent system, focusing on the modular, testable design implemented during the refactoring.

## Overview

OxideAgent follows a modular architecture with clear separation of concerns. The system is divided into core business logic, interface implementations, and configuration management.

## High-Level Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Interface     │◄──►│   Core Logic     │◄──►│ Configuration   │
│  (TUI, Web,     │    │                  │    │                 │
│  Telegram, etc) │    │  - Agents        │    │  - Config       │
│                 │    │  - Orchestrator  │    │  - Container    │
│                 │    │  - Tools         │    │                 │
│                 │    │  - Session       │    │                 │
│                 │    │  - Events        │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Core Components

### Core Module (`src/core/`)

The core module contains the business logic of the application:

- **agents/**: Agent implementations and management
- **llm/**: Language model integration (Ollama)
- **tools/**: Tool implementations and registry
- **session/**: Session state management and persistence
- **orchestrator.rs**: Core orchestration logic
- **container.rs**: Dependency injection container
- **events.rs**: Event system implementation
- **interface.rs**: Interface abstraction traits

### Interfaces Module (`src/interfaces/`)

Interface implementations, currently only TUI:

- **tui/**: Terminal User Interface implementation
- **mod.rs**: Interface trait implementations

### Configuration (`src/config.rs`)

Centralized configuration management with validation.

### Events System

The enhanced event system provides:
- `EventBus`: For asynchronous communication between components
- `EventFilter`: For routing events based on source, destination, and type
- `EventType`: Comprehensive event types for all application needs

## Dependency Injection Container

The `Container` struct manages dependencies between components:

```rust
pub struct Container {
    config: Arc<Config>,
    agent: Option<Agent>,
    tool_registry: Option<ToolRegistry>,
    session_manager: Option<SessionManager>,
}
```

## Interface Abstraction

The system uses a trait-based interface abstraction:

```rust
#[async_trait]
pub trait Interface: InputHandler + OutputHandler + EventEmitter + Send {
    async fn init(&mut self) -> Result<()>;
    async fn run(&mut self) -> Result<()>;
    async fn cleanup(&mut self) -> Result<()>;
    fn get_session_history(&self) -> Vec<ChatMessage>;
    fn get_session_name(&self) -> String;
}
```

## Event-Driven Architecture

Components communicate through events using the event bus system:

- **InputHandler**: Handles user input
- **OutputHandler**: Sends output to the interface
- **EventEmitter**: Manages event channels

## Testing Architecture

The testing infrastructure includes:

- **Unit tests**: Located in `tests/unit/`, testing individual modules
- **Integration tests**: Located in `tests/integration/`, testing component interactions
- **Test utilities**: Located in `tests/utils/`, providing mock objects and utilities

## Module Interactions

```
┌─────────────┐
│   Agent     │
└──────┬──────┘
       │ Uses
┌──────▼──────┐    ┌─────────────┐
│ Orchestrator│◄───┤   Events    │
└──────┬──────┘    └─────────────┘
       │               ▲
       │ Uses          │ Uses
┌──────▼──────┐       │
│    Tools    │       │
└─────────────┘       │
                      │
        ┌─────────────┼─────────────┐
        │             │             │
┌───────▼───┐ ┌───────▼───┐ ┌─────▼─────┐
│   TUI     │ │   Web     │ │ Telegram  │
│Interface  │ │Interface  │ │Interface  │
└───────────┘ └───────────┘ └───────────┘
```

## Configuration Management

Configuration is managed through a centralized system:

- **Config struct**: Single source of configuration data
- **Validation**: Configuration is validated at startup
- **InterfaceType**: Enum to specify which interface to use

## Error Handling

The system uses Rust's standard error handling patterns with `anyhow::Result` for most functions and specific error types where needed.

## Future Extensibility

The architecture supports:

- Adding new interface types (Web UI, Telegram bot)
- Adding new tools through the trait system
- Integration with Model Context Protocol (MCP) servers
- Multiple agent implementations
- Advanced workflow management