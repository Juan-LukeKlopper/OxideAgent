# OxideAgent

A local-first, feature-rich AI agent designed to run on your machine. This project provides a powerful command-line interface (CLI) to interact with local language models through the Ollama platform.

## About The Project

OxideAgent is a Rust-based AI agent that allows you to chat with local language models directly from your terminal. The goal is to create a powerful and extensible agent that can perform a variety of tasks, from simple chat to complex, multi-step operations with tool integration.

### Core Features

*   **Local-First:** All processing is done on your local machine, ensuring privacy and control over your data.
*   **Ollama Integration:** Seamlessly connects to the Ollama platform to leverage a wide range of local language models.
*   **Multi-Agent Support:** Easily switch between different AI "personalities" or agents, each configured with a specific model.
*   **Interactive Chat:** Engage in continuous conversations with the selected agent, with support for session memory.
*   **Streaming Responses:** Get real-time feedback from the agent as it generates a response.

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
    ollama pull tinydolphin
    ollama pull phi3
    ```
3.  Build the project
    ```sh
    cargo build --release
    ```

## Usage

To start a chat session with the default agent, run the following command:

```sh
cargo run
```

You can also select a specific agent using the `--agent` flag:

```sh
cargo run -- --agent reviewer
```

To see a list of available agents, use the `--help` flag:

```sh
cargo run -- --help
```

To exit the chat, type `/exit`.

## Project Roadmap

Future development will focus on expanding the agent's capabilities. Key features on the roadmap include:

*   **Tool Integration:** Allowing the agent to interact with external tools and APIs (e.g., writing to files, running shell commands).
*   **Persistent Memory:** Enabling the agent to remember context and state across sessions.
*   **TUI Makeover:** Transforming the CLI into a more interactive and user-friendly Terminal User Interface (TUI).

## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1.  Fork the Project
2.  Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3.  Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4.  Push to the Branch (`git push origin feature/AmazingFeature`)
5.  Open a Pull Request

## License

Distributed under the MIT License.
