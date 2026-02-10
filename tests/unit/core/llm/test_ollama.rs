use OxideAgent::core::llm::client::LlmClient;
use OxideAgent::core::llm::ollama::{OllamaClient, list_models};
use OxideAgent::types::{AppEvent, ChatMessage, Tool, ToolFunctionDefinition};
use httpmock::prelude::*;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_list_models_success() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/tags");
        then.status(200)
            // ... (unchanged)
            .header("content-type", "application/json")
            .json_body(json!({
                "models": [
                    {"name": "model1"},
                    {"name": "model2"}
                ]
            }));
    });

    let client = Client::builder().no_proxy().build().unwrap();
    let base_url = server.base_url();
    let result = list_models(&client, &base_url).await;

    mock.assert();
    assert!(result.is_ok());
    let models = result.unwrap();
    assert_eq!(models, vec!["model1", "model2"]);
}

// ... helper for other list_models tests ...
#[tokio::test]
async fn test_list_models_empty() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/tags");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "models": []
            }));
    });
    let client = Client::builder().no_proxy().build().unwrap();
    let base_url = server.base_url();
    let result = list_models(&client, &base_url).await;
    mock.assert();
    assert!(result.is_ok());
    let models = result.unwrap();
    assert!(models.is_empty());
}

#[tokio::test]
async fn test_list_models_error() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/tags");
        then.status(500);
    });
    let client = Client::builder().no_proxy().build().unwrap();
    let base_url = server.base_url();
    let result = list_models(&client, &base_url).await;
    mock.assert();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_chat_non_streaming_success() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/api/chat");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "message": {
                    "content": "Hello, world!",
                    "tool_calls": []
                }
            }));
    });

    let base_url = server.base_url();
    let client = OllamaClient::new(&base_url);
    let (tx, _) = mpsc::channel(1);
    let history = vec![ChatMessage::user("Hello")];
    let tools = vec![];
    let result = client.chat("model1", &history, &tools, false, tx).await;

    mock.assert();
    assert!(result.is_ok());
    let response = result.unwrap().unwrap();
    assert_eq!(response.content, "Hello, world!");
    assert!(response.tool_calls.is_none());
}

#[tokio::test]
async fn test_send_chat_non_streaming_with_tools() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/api/chat");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "message": {
                    "content": "",
                    "tool_calls": [
                        {
                            "function": {
                                "name": "test_tool",
                                "arguments": "{ \"arg1\": \"value1\" }"
                            }
                        }
                    ]
                }
            }));
    });

    let base_url = server.base_url();
    let client = OllamaClient::new(&base_url);
    let (tx, _) = mpsc::channel(1);
    let history = vec![ChatMessage::user("Use the test tool")];
    let tools = vec![Tool {
        r#type: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: json!({}),
        },
    }];
    let result = client.chat("model1", &history, &tools, false, tx).await;

    mock.assert();
    assert!(result.is_ok());
    let response = result.unwrap().unwrap();
    assert_eq!(response.content, "");
    assert!(response.tool_calls.is_some());
    let tool_calls = response.tool_calls.unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].function.name, "test_tool");
}

#[tokio::test]
async fn test_send_chat_streaming_success() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/api/chat");
        let body = "{\"message\":{\"content\":\"Hello\"}}\n{\"message\":{\"content\":\", \"}}\n{\"message\":{\"content\":\"world!\"}}\n{\"done\":true}\n";
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let base_url = server.base_url();
    let client = OllamaClient::new(&base_url);
    let (tx, mut rx) = mpsc::channel(10);
    let history = vec![ChatMessage::user("Hello")];
    let tools = vec![];
    let chat_future = client.chat("model1", &history, &tools, true, tx);

    let mut received_content = String::new();
    let mut stream_ended = false;

    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                AppEvent::AgentStreamChunk(chunk) => {
                    received_content.push_str(&chunk);
                }
                AppEvent::AgentStreamEnd => {
                    stream_ended = true;
                    break;
                }
                _ => {} // Ignore other events
            }
        }
        (received_content, stream_ended)
    });

    let result = chat_future.await;

    mock.assert();
    assert!(result.is_ok());
    let response = result.unwrap().unwrap();
    assert_eq!(response.content, "Hello, world!");
    assert!(response.tool_calls.is_none());

    let (final_content, final_stream_ended) = handle.await.unwrap();
    assert_eq!(final_content, "Hello, world!");
    assert!(final_stream_ended);
}
