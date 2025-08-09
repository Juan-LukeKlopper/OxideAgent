use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "LLM CLI", about = "Chat with local Ollama models")]
pub struct Args {
    #[arg(long, value_enum, default_value = "code")]
    pub agent: AgentType,

    #[arg(long, default_value_t = false)]
    pub no_stream: bool,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum AgentType {
    Code,
    Reviewer,
    Doc,
}

impl AgentType {
    pub fn model(&self) -> &'static str {
        match self {
            AgentType::Code => "qwen3:4b",
            AgentType::Reviewer => "tinydolphin",
            AgentType::Doc => "granite3.3",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            AgentType::Code => "Code Assistant",
            AgentType::Reviewer => "Reviewer",
            AgentType::Doc => "Doc Writer",
        }
    }
}
