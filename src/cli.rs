use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "LLM CLI", about = "Chat with local Ollama models")]
pub struct Args {
    #[arg(long, value_enum)]
    pub agent: Option<AgentType>,

    #[arg(long)]
    pub no_stream: Option<bool>,

    #[arg(
        long,
        help = "Specify a session name to load/save state to a named session file"
    )]
    pub session: Option<String>,

    #[arg(long, help = "List all available sessions")]
    pub list_sessions: Option<bool>,

    #[arg(long, help = "URL of an MCP server to connect to")]
    pub mcp_server: Option<String>,

    #[arg(long, help = "Authentication token for the MCP server")]
    pub mcp_auth_token: Option<String>,

    #[arg(long, value_enum, help = "Interface type to use")]
    pub interface: Option<InterfaceType>,

    #[arg(
        long,
        value_name = "CONFIG_FILE",
        help = "Path to a configuration file (JSON, YAML, or TOML format)"
    )]
    pub config: Option<String>,

    #[arg(long, help = "The base URL for the LLM API")]
    pub llm_api_base: Option<String>,

    #[arg(long, help = "The API key for the LLM API")]
    pub llm_api_key: Option<String>,

    #[arg(long, help = "The model to use for the LLM")]
    pub llm_model: Option<String>,
}

#[derive(ValueEnum, Debug, Clone, PartialEq)]
pub enum AgentType {
    Qwen,
    Llama,
    Granite,
}

#[derive(ValueEnum, Debug, Clone, PartialEq)]
pub enum InterfaceType {
    Tui,
    // In the future we could add Web, Telegram, etc.
}

impl AgentType {
    pub fn name(&self) -> &'static str {
        match self {
            AgentType::Qwen => "Qwen",
            AgentType::Llama => "Llama",
            AgentType::Granite => "Granite",
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentType::Qwen => "You are a Rust programming expert.",
            AgentType::Llama => "You are a helpful assistant.",
            AgentType::Granite => "You are a helpful assistant.",
        }
    }

    #[allow(dead_code)] // Keep for future use or external API
    pub fn model_keyword(&self) -> &'static str {
        match self {
            AgentType::Qwen => "qwen",
            AgentType::Llama => "llama",
            AgentType::Granite => "granite",
        }
    }
}
