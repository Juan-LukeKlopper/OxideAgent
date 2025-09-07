use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "LLM CLI", about = "Chat with local Ollama models")]
pub struct Args {
    #[arg(long, value_enum, default_value = "qwen")]
    pub agent: AgentType,

    #[arg(long, default_value_t = false)]
    pub no_stream: bool,

    #[arg(long, help = "Specify a session name to load/save state to a named session file")]
    pub session: Option<String>,

    #[arg(long, help = "List all available sessions", default_value_t = false)]
    pub list_sessions: bool,

    #[arg(long, help = "URL of an MCP server to connect to")]
    pub mcp_server: Option<String>,

    #[arg(long, help = "Authentication token for the MCP server")]
    pub mcp_auth_token: Option<String>,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum AgentType {
    Qwen,
    Llama,
    Granite,
}

impl AgentType {
    pub fn model(&self) -> &'static str {
        match self {
            AgentType::Qwen => "qwen3:4b",
            AgentType::Llama => "llama3.2",
            AgentType::Granite => "smolLM2",
        }
    }

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
}

