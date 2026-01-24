pub mod client;
pub mod ollama;

use crate::config::LLMConfig;
use anyhow::Result;
use client::LlmClient;
use ollama::OllamaClient;

pub fn llm_client_factory(config: &LLMConfig) -> Result<Box<dyn LlmClient>> {
    match config.provider.as_str() {
        "ollama" => Ok(Box::new(OllamaClient::new(&config.api_base))),
        // Future providers will go here
        provider => Err(anyhow::anyhow!("Unsupported LLM provider: {}", provider)),
    }
}
