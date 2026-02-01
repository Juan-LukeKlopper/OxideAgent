use OxideAgent::core::llm::client::LlmClient;
use OxideAgent::core::llm::ollama::OllamaClient;
use OxideAgent::types::{AppEvent, ChatMessage, Tool};
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a simple tool to test with
    let test_tool = Tool {
        r#type: "function".to_string(),
        function: OxideAgent::types::ToolFunctionDefinition {
            name: "test_tool".to_string(),
            description: "A test tool for debugging".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string",
                        "description": "A test parameter"
                    }
                },
                "required": ["param1"]
            }),
        },
    };

    let tools = vec![test_tool];

    // Create a simple message history
    let history = vec![ChatMessage::user("Can you use the test tool?")];

    // Create channel for app events
    let (tx, _rx) = mpsc::channel::<AppEvent>(32);

    // Create HTTP client
    let client = Client::new();

    // Fetch available models
    let available_models =
        match OxideAgent::core::llm::ollama::list_models(&client, "http://localhost:11434").await {
            Ok(models) => models,
            Err(e) => {
                println!("Error fetching Ollama models: {}", e);
                return Ok(());
            }
        };

    let model = available_models.first().unwrap();

    // Send the chat request and see what gets logged
    println!("Sending test chat request...");

    let ollama_client = OllamaClient::new("http://localhost:11434");

    match ollama_client.chat(model, &history, &tools, false, tx).await {
        Ok(response) => {
            println!("Got response: {:?}", response);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    Ok(())
}
