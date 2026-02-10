# OxideAgent Architecture

This document describes the architecture of the OxideAgent system, focusing on the modular, testable design with multi-agent support.

## Overview

OxideAgent follows a modular architecture with clear separation of concerns. The system is divided into core business logic, interface implementations, and configuration management. It supports multiple concurrent agents, each with their own sessions, permissions, and model configurations.

## High-Level Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Interface     │◄──►│   Core Logic     │◄──►│ Configuration   │
│  (TUI, Web,     │    │                  │    │                 │
│ Telegram,Discord)│    │  - Orchestrator  │    │  - Config       │
│                 │    │  - MultiAgent    │    │  - Container    │
│                 │    │  - Tools         │    │                 │
│                 │    │  - Session       │    │                 │
│                 │    │  - LLM Client    │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Core Components

### Core Module (`src/core/`)

The core module contains the business logic of the application:

- **agents/**: Individual agent implementation with history management
- **llm/**: Language model integration with `LlmClient` trait abstraction
  - `client.rs`: `LlmClient` trait definition
  - `ollama.rs`: `OllamaClient` implementation for Ollama API
  - `mod.rs`: Factory function `llm_client_factory` for client creation
- **multi_agent_manager.rs**: Manages multiple concurrent agents with:
  - Individual session states per agent
  - Individual tool permissions (global and session-specific)
  - Individual history and model configurations
  - Async communication via broadcast channels
- **tools/**: Tool implementations and registry
- **session/**: Session state management and persistence
- **orchestrator.rs**: Routes events to active agent via `MultiAgentManager`
- **container.rs**: Dependency injection container
- **events.rs**: Event system implementation
- **interface.rs**: Interface abstraction traits

### Multi-Agent System

The multi-agent system allows running multiple AI agents concurrently:

```rust
pub struct MultiAgentManager {
    agents: Arc<RwLock<HashMap<AgentId, AgentHandle>>>,
    tool_registry: ToolRegistry,
    system_prompt: String,
    llm_config: LLMConfig,
    event_tx: broadcast::Sender<AppEvent>,
}
```

Each agent runs in its own async task with:
- Dedicated message channel
- Individual session state
- Individual tool permissions
- Own conversation history

### LLM Client Abstraction

The `LlmClient` trait provides a unified interface for all LLM backends:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(
        &self,
        model: &str,
        history: &[ChatMessage],
        tools: &[ApiTool],
        stream: bool,
        tx: mpsc::Sender<AppEvent>,
    ) -> anyhow::Result<Option<ChatMessage>>;
}
```

Currently implemented:
- `OllamaClient`: For local Ollama models

### Interfaces Module (`src/interfaces/`)

Interface implementations, currently TUI with Web/Telegram/Discord scaffolding in config+CLI:

- **tui/**: Terminal User Interface implementation
- **mod.rs**: Interface trait implementations

### Configuration (`src/config.rs`)

Centralized configuration management including:
- Agent configuration
- Multi-agent settings
- LLM provider configuration
- MCP (Model Context Protocol) settings

## Module Interactions

```
                      ┌─────────────────┐
                      │  Orchestrator   │
                      └────────┬────────┘
                               │ Delegates
                      ┌────────▼────────┐
                      │ MultiAgentMgr   │
                      └────────┬────────┘
           ┌───────────────────┼───────────────────┐
           │                   │                   │
    ┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
    │   Agent 1   │     │   Agent 2   │     │   Agent N   │
    │  (Qwen)     │     │  (Llama)    │     │  (Granite)  │
    └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
           │                   │                   │
    ┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
    │  LlmClient  │     │  LlmClient  │     │  LlmClient  │
    └─────────────┘     └─────────────┘     └─────────────┘
```

## Event-Driven Architecture

Components communicate through events using the event bus system:

- **AppEvent**: Enum covering all application events
- **broadcast::Sender**: For multi-agent event distribution
- **mpsc::channel**: For TUI communication

Key events:
- `UserInput`: User message to active agent
- `SwitchAgent`: Switch to a different agent
- `ToolApproval`: Tool execution approval
- `AgentMessage`: Response from agent

## Testing Architecture

The testing infrastructure includes:

- **Unit tests**: Located in `tests/unit/`
- **Integration tests**: Located in `tests/integration/`
- **Test utilities**: Located in `tests/utils/`
- **Mocks**: `MockOllamaClient` for testing LLM interactions

## Future Extensibility

The architecture supports:

- Adding new LLM providers (OpenAI, Anthropic, etc.) via `LlmClient` trait
- Adding new interface types (Web UI, Telegram bot, Discord bot)
- Adding new tools through the trait system
- Integration with Model Context Protocol (MCP) servers
- Advanced workflow management with multiple agents