# OxideAgent

A local-first, feature-rich AI agent designed to run on your machine. This project provides a powerful command-line interface (CLI) to interact with local language models through the Ollama platform.

## About The Project

OxideAgent is a Rust-based AI agent that allows you to chat with local language models directly from your terminal. The goal is to create a powerful and extensible agent that can perform a variety of tasks, from simple chat to complex, multi-step operations with tool integration.

### Core Features

*   **Local-First:** All processing is done on your local machine, ensuring privacy and control over your data.
*   **Ollama Integration:** Seamlessly connects to the Ollama platform to leverage a wide range of local language models.
*   **Multi-Agent Support:** Easily switch between different agents, each configured with a specific model.
*   **Persistent Sessions:** The agent remembers your conversation history, allowing you to stop and resume long-running tasks at any time.
*   **Streaming Responses:** Get real-time feedback from the agent as it generates a response.
*   **Extensible Tool System:** Uses a scalable, trait-based system for adding new tools.
*   **Native Tool Calling:** Leverages Ollama's native tool-calling API for reliable and structured tool interactions.
*   **Tool Approval System:** A security-focused workflow requires user approval before executing any tool.
*   **Advanced TUI Interface:** Features a Terminal User Interface with collapsible sections for reasoning and tool outputs.
*   **Thinking Process Visualization:** Clearly separates agent reasoning from final responses with expandable/collapsible sections.

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

To exit the chat, type `/exit`.

### Available Agents

1.  **Qwen** (`--agent qwen`): Uses the `qwen3:4b` model. The default agent.
2.  **Llama** (`--agent llama`): Uses the `llama3.2` model.
3.  **Granite** (`--agent granite`): Uses the `granite3.3` model.

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

## Project Roadmap

The project has a solid foundation with the following features already implemented:
*   Basic Ollama connection
*   Interactive multi-agent chat
*   File operations and shell command execution
*   A "smart" native tool-calling system
*   An orchestrator with persistent memory for resumable sessions
*   Advanced TUI with collapsible sections for better visualization

Future development will focus on expanding the agent's capabilities:
*   **Multi-Session Management:** Allow for creating and switching between multiple named sessions.
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