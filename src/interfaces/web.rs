//! Minimal web interface scaffold (Phase W1-W2 foundation).

use crate::core::interface::{EventEmitter, InputHandler, Interface, OutputHandler};
use crate::types::{AppEvent, ChatMessage};
use anyhow::Result;
use tokio::sync::mpsc;

#[allow(dead_code)]
pub struct WebInterface {
    rx: Option<mpsc::Receiver<AppEvent>>,
    tx: mpsc::Sender<AppEvent>,
    session_name: String,
    session_history: Vec<ChatMessage>,
}

impl WebInterface {
    pub fn new(
        rx: mpsc::Receiver<AppEvent>,
        tx: mpsc::Sender<AppEvent>,
        session_name: String,
        session_history: Vec<ChatMessage>,
    ) -> Self {
        Self {
            rx: Some(rx),
            tx,
            session_name,
            session_history,
        }
    }
}

#[async_trait::async_trait]
impl InputHandler for WebInterface {
    async fn handle_input(&mut self, input: String) -> Result<()> {
        self.tx.send(AppEvent::UserInput(input)).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl OutputHandler for WebInterface {
    async fn send_output(&mut self, output: AppEvent) -> Result<()> {
        self.tx.send(output).await?;
        Ok(())
    }
}

impl EventEmitter for WebInterface {
    fn get_event_sender(&self) -> mpsc::Sender<AppEvent> {
        self.tx.clone()
    }

    fn get_event_receiver(&mut self) -> mpsc::Receiver<AppEvent> {
        self.rx.take().unwrap_or_else(|| {
            let (_tx, rx) = mpsc::channel(1);
            rx
        })
    }
}

#[async_trait::async_trait]
impl Interface for WebInterface {
    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_session_history(&self) -> Vec<ChatMessage> {
        self.session_history.clone()
    }

    fn get_session_name(&self) -> String {
        self.session_name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::WebInterface;
    use crate::core::interface::InputHandler;
    use crate::types::AppEvent;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn handle_input_forwards_user_input_event() {
        let (_orchestrator_tx, interface_rx) = mpsc::channel(8);
        let (interface_tx, mut orchestrator_rx) = mpsc::channel(8);
        let mut interface = WebInterface::new(
            interface_rx,
            interface_tx,
            "session-a".to_string(),
            Vec::new(),
        );

        interface.handle_input("hello".to_string()).await.unwrap();

        let event = orchestrator_rx.recv().await;
        assert!(matches!(event, Some(AppEvent::UserInput(input)) if input == "hello"));
    }
}
