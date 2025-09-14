# OxideAgent ✅

[![CI](https://github.com/your-username/OxideAgent/workflows/CI/badge.svg)](https://github.com/your-username/OxideAgent/actions)

A local-first, feature-rich AI agent written in Rust that runs on your machine and connects to Ollama.

## Overview

OxideAgent is a sophisticated AI agent that provides a powerful command-line interface to interact with local language models through the Ollama platform. It offers a unique blend of local-first architecture with cloud-capable extensibility through MCP server integration.

This project has undergone a comprehensive refactoring to improve its architecture, modularity, and maintainability:
- **Modular Architecture**: Clean separation of core logic and interface implementations
- **Interface Abstraction**: Support for multiple interface types (TUI, future Web, Telegram, etc.)
- **Configuration Management**: Centralized configuration with validation
- **Dependency Injection**: Service container for managing component dependencies
- **Enhanced Event System**: Robust event system for communication between components
- **Testing Infrastructure**: Comprehensive testing framework with unit and integration tests

## Features

- **Local-First Architecture**: All processing happens on your machine, ensuring privacy and control over your data
- **Ollama Integration**: Seamlessly connects to the Ollama platform to leverage a wide range of local language models
- **Multi-Agent Support**: Easily switch between different agents, each configured with a specific model
- **Persistent Sessions**: The agent remembers your conversation history, allowing you to stop and resume long-running tasks at any time
- **Streaming Responses**: Get real-time feedback from the agent as it generates a response
- **Extensible Tool System**: Uses a scalable, trait-based system for adding new tools
- **Native Tool Calling**: Leverages Ollama's native tool-calling API for reliable and structured tool interactions
- **Tool Approval System**: A security-focused workflow requires user approval before executing any tool
- **Advanced TUI Interface**: Features a Terminal User Interface with collapsible sections for reasoning and tool outputs
- **Thinking Process Visualization**: Clearly separates agent reasoning from final responses with expandable/collapsible sections
- **Multi-Session Management**: Create and switch between multiple named sessions with persistent history
- **Session History Restoration**: Automatically restores previous conversations when loading a session

## Technologies Used

- **Language**: Rust (2024 edition)
- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest
- **Terminal UI**: Ratatui with Crossterm
- **Serialization**: Serde with JSON support
- **CLI Parsing**: Clap
- **Input Handling**: tui-input

## Installation

1. Clone the repo
   ```sh
   git clone https://github.com/your_username/OxideAgent.git
   ```

2. Install the required Ollama models
   ```sh
   ollama pull qwen3:4b
   ollama pull llama3.2
   ollama pull granite3.3
   ```

3. Build the project
   ```sh
   cargo build --release
   ```

## Usage

To start a chat session with the default agent (`qwen`):
```sh
cargo run
```

Select a specific agent using the `--agent` flag:
```sh
cargo run -- --agent llama
```

Start a named session:
```sh
cargo run -- --session my_project
```

List all sessions:
```sh
cargo run -- --list-sessions
```

See all available options:
```sh
cargo run -- --help
```

## Available Agents

1. **Qwen** (`--agent qwen`): Uses the `qwen3:4b` model. The default agent.
2. **Llama** (`--agent llama`): Uses the `llama3.2` model.
3. **Granite** (`--agent granite`): Uses the `granite3.3` model.

## Tool Capabilities

The agent has access to several tools that allow it to interact with your system:

1. **write_file**: Write content to a file on your system.
2. **read_file**: Read content from a file on your system.
3. **run_shell_command**: Execute shell commands on your system.

When the agent wants to use a tool, you'll be prompted to approve its execution for security.

## TUI Features

The Terminal User Interface provides an enhanced chat experience with several advanced features:

1. **Collapsible Reasoning Sections**: Agent thinking processes are displayed in expandable/collapsed sections marked with `[Click to expand/collapse]`
2. **Collapsible Tool Outputs**: Tool execution results are also displayed in expandable/collapsed sections by default
3. **Real-time Streaming**: Watch responses appear character-by-character as they're generated
4. **Mouse Support**: Click on section headers to expand or collapse content
5. **Improved Layout**: Better organized chat history with clear visual separation between different message types
6. **Session Management**: View and switch between sessions directly from the TUI
7. **Help System**: Press `Ctrl+o` to display all available commands and shortcuts

### TUI Keyboard Shortcuts

- **Ctrl+q**: Quit the application
- **Ctrl+a**: Toggle agent/session switcher
- **Ctrl+o**: Show help message with all commands
- **Mouse Click**: Expand/collapse reasoning and tool output sections
- **Tool Approval Options** (when prompted):
  - 1: Allow tool execution
  - 2: Always allow this tool
  - 3: Always allow this tool for this session
  - 4: Deny tool execution

### Session Commands

- **/switch <session_name>**: Switch to a different session from within the TUI
- **Ctrl+s**: List all available sessions

## Development

This project follows a modular architecture with clear separation of concerns:

### Code Structure

```
├── src/
│   ├── agents/           # Agent implementations
│   ├── cli.rs           # Command-line interface parsing
│   ├── main.rs          # Main entry point
│   ├── ollama.rs        # Ollama API integration
│   ├── orchestrator.rs  # Core orchestration logic
│   ├── tools.rs         # Tool implementations and registry
│   ├── tui/             # Terminal User Interface components
│   └── types.rs         # Shared data structures
├── Cargo.toml           # Project dependencies and metadata
├── README.md            # Project documentation
├── GOAL.md              # Project goals and development roadmap
├── cool_tricks.md       # Notes on TUI enhancements
└── session*.json        # Persistent session files
```

### Testing

The project includes both unit and integration tests. To run the tests:

```sh
cargo test
```

### CI/CD

The project uses GitHub Actions for continuous integration. The workflow includes:
- Code formatting checks
- Clippy linting
- Building the project
- Running tests
- Building documentation

See `.github/workflows/ci.yml` for details.

## Planned Expansions

The current implementation focuses on the Terminal User Interface (TUI), but the architecture is designed to be extensible to other interfaces and platforms:

### Web UI

The project is designed with a clean separation between core logic and UI presentation, making it straightforward to add a web-based interface:

1. **Architecture**: The core agent logic is independent of the UI layer, communicating through event channels.
2. **API Layer**: A web API can be added to expose the same events and functionality over HTTP/WebSocket.
3. **Frontend**: Modern web frameworks (React, Vue.js, etc.) can consume the API to provide a rich web interface.
4. **Tauri Integration**: The project can be wrapped in Tauri to create a desktop application with a web-based frontend.

### Telegram Bots

The modular architecture allows for easy integration with messaging platforms like Telegram:

1. **Bot Implementation**: A Telegram bot interface can be added as a new entry point alongside the TUI.
2. **Session Management**: Each Telegram user or chat can have its own session, similar to named sessions in the TUI.
3. **Agent Spawning**: Users can spawn and interact with different agents through Telegram commands.
4. **Tool Approval**: The security model can be adapted to work with Telegram's messaging system.

### MCP Server Integration

The project is designed to integrate with Model Context Protocol (MCP) servers:

1. **External Tools**: Connect to MCP servers for advanced tooling (Strava, Garmin, etc.).
2. **Local MCP Servers**: Ability to spin up local MCP servers for specific integrations.
3. **Smart Tool Inclusion**: Dynamically select tools based on the agent's task to avoid exposing unrelated capabilities.

## Project Roadmap

Current features:
- Basic Ollama connection
- Interactive multi-agent chat
- File operations and shell command execution
- A "smart" native tool-calling system
- An orchestrator with persistent memory for resumable sessions
- Advanced TUI with collapsible sections for better visualization
- Multi-session management with named sessions
- Session history restoration

Future development will focus on:
- **MCP Server Integration**: Connect to Model Context Protocol servers for advanced tooling
- **Smart Tool & Prompt Inclusion**: Dynamically select tools and system prompts based on the agent's task
- **Advanced Workflow Management**: Handle complex, multi-step operations with better planning and error handling
- **Web UI Implementation**: Add a web-based interface for broader accessibility
- **Telegram Bot Integration**: Enable interaction with agents through Telegram
- **Additional Platform Support**: Expand to other messaging platforms and interfaces

## Contributing

Contributions are welcome. The project follows standard Rust development practices:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is distributed under the MIT License.

## Getting Started

To get a local copy up and running follow these simple steps.

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   [Ollama](https://ollama.ai/)

### Installation

1.  Clone the repo
    ```sh
    git clone https://github.com/your_username/OxideAgent.git
    ```
2.  Install the required Ollama models
    ```sh
    ollama pull qwen3:4b
    ollama pull llama3.2
    ollama pull granite3.3
    ```
3.  Build the project
    ```sh
    cargo build --release
    ```

## Usage

To start a chat session with the default agent (`qwen`), run the following command:

```sh
cargo run
```

The agent will welcome you back if it finds a previous session file (`session.json`). To start a fresh session, you can delete this file.

You can also select a specific agent using the `--agent` flag:

```sh
cargo run -- --agent llama
```

To see a list of available agents, use the `--help` flag:

```sh
cargo run -- --help
```

To exit the chat, press `Ctrl+q`.

### Available Agents

1.  **Qwen** (`--agent qwen`): Uses the `qwen3:4b` model. The default agent.
2.  **Llama** (`--agent llama`): Uses the `llama3.2` model.
3.  **Granite** (`--agent granite`): Uses the `granite3.3` model.

### Multi-Session Management

OxideAgent supports multiple named sessions, allowing you to work on different tasks simultaneously:

*   Start a named session: `cargo run -- --session my_project`
*   List all sessions: `cargo run -- --list-sessions`
*   Switch between sessions within the TUI using `/switch session_name`

### Tool Capabilities

The agent has access to several tools that allow it to interact with your system:

1.  **write_file**: Write content to a file on your system.
2.  **read_file**: Read content from a file on your system.
3.  **run_shell_command**: Execute shell commands on your system.

When the agent wants to use a tool, you'll be prompted to approve its execution for security.

## TUI Features

The Terminal User Interface provides an enhanced chat experience with several advanced features:

*   **Collapsible Reasoning Sections**: Agent thinking processes are displayed in expandable/collapsible sections marked with `[Click to expand/collapse]`
*   **Collapsible Tool Outputs**: Tool execution results are also displayed in expandable/collapsible sections by default
*   **Real-time Streaming**: Watch responses appear character-by-character as they're generated
*   **Mouse Support**: Click on section headers to expand or collapse content
*   **Improved Layout**: Better organized chat history with clear visual separation between different message types
*   **Session Management**: View and switch between sessions directly from the TUI
*   **Help System**: Press `Ctrl+o` to display all available commands and shortcuts

### TUI Keyboard Shortcuts

*   **Ctrl+q**: Quit the application
*   **Ctrl+s**: List available sessions
*   **Ctrl+o**: Show help message with all commands
*   **Mouse Click**: Expand/collapse reasoning and tool output sections
*   **Tool Approval Options** (when prompted):
    *   1: Allow tool execution
    *   2: Always allow this tool
    *   3: Always allow this tool for this session
    *   4: Deny tool execution

### Session Commands

*   **/switch <session_name>**: Switch to a different session from within the TUI
*   **Ctrl+s**: List all available sessions

## Project Roadmap

The project has a solid foundation with the following features already implemented:
*   Basic Ollama connection
*   Interactive multi-agent chat
*   File operations and shell command execution
*   A "smart" native tool-calling system
*   An orchestrator with persistent memory for resumable sessions
*   Advanced TUI with collapsible sections for better visualization
*   Multi-session management with named sessions
*   Session history restoration

Future development will focus on expanding the agent's capabilities:
*   **MCP Server Integration:** Connect to Model Context Protocol servers for advanced tooling.
*   **Smart Tool & Prompt Inclusion:** Dynamically select tools and system prompts based on the agent's task.
*   **Advanced Workflow Management:** Handle complex, multi-step operations with better planning and error handling.

## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1.  Fork the Project
2.  Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3.  Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4.  Push to the Branch (`git push origin feature/AmazingFeature`)
5.  Open a Pull Request

## License

Distributed under the MIT License.
