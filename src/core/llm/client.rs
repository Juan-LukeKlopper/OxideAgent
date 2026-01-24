use crate::types::{AppEvent, ChatMessage, Tool};
use async_trait::async_trait;
use tokio::sync::mpsc;
use std::fmt::Debug;

/// Trait defining the interface for LLM clients.
/// This allows the agent to interact with different LLM providers (Ollama, OpenAI, etc.)
/// without knowing the implementation details.
#[async_trait]
pub trait LlmClient: Send + Sync + Debug {
    /// Send a chat request to the LLM.
    ///
    /// # Arguments
    /// * `model` - The model to use (e.g., "qwen2.5-coder", "gpt-4")
    /// * `history` - The conversation history
    /// * `tools` - Available tools for the LLM
    /// * `stream` - Whether to stream the response
    /// * `tx` - Channel to send application events (chunks, errors, etc.)
    ///
    /// # Returns
    /// * `Result<Option<ChatMessage>>` - The assistant's response message (if not streaming, or collected after stream)
    async fn chat(
        &self,
        model: &str,
        history: &[ChatMessage],
        tools: &[Tool],
        stream: bool,
        tx: mpsc::Sender<AppEvent>,
    ) -> anyhow::Result<Option<ChatMessage>>;
}
