use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "LLM CLI", about = "Chat with local Ollama models")]
pub struct Args {
    #[arg(long, value_enum, default_value = "qwen")]
    pub agent: AgentType,

    #[arg(long, default_value_t = false)]
    pub no_stream: bool,
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
            AgentType::Granite => "granite3.3",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            AgentType::Qwen => "Qwen",
            AgentType::Llama => "Llama",
            AgentType::Granite => "Granite",
        }
    }
}