//! Dependency injection container for the OxideAgent system.
//!
//! This module implements a service container for managing dependencies
//! between components in the application.

use crate::config::OxideConfig;
use crate::core::agents::Agent;
use crate::core::orchestrator::Orchestrator;
use crate::core::session::SessionManager;
use crate::core::tools::ToolRegistry;
use crate::types::AppEvent;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Service container for managing dependencies
pub struct Container {
    config: Arc<OxideConfig>,
    agent: Option<Agent>,
    tool_registry: Option<ToolRegistry>,
    #[allow(dead_code)]
    session_manager: Option<SessionManager>,
}

impl Container {
    /// Create a new container with the given configuration
    pub fn new(config: OxideConfig) -> Self {
        Self {
            config: Arc::new(config),
            agent: None,
            tool_registry: None,
            session_manager: None,
        }
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &OxideConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration
    #[allow(dead_code)]
    pub fn config_mut(&mut self) -> &mut OxideConfig {
        Arc::get_mut(&mut self.config).expect("Config is shared")
    }

    /// Build the agent
    pub fn build_agent(&mut self) -> Result<&mut Agent> {
        if self.agent.is_none() {
            let agent = Agent::new(&self.config.agent.name, &self.config.agent.model);
            self.agent = Some(agent);
        }
        Ok(self.agent.as_mut().unwrap())
    }

    /// Build the tool registry
    pub fn build_tool_registry(&mut self) -> Result<&mut ToolRegistry> {
        if self.tool_registry.is_none() {
            let mut tool_registry = ToolRegistry::new();
            // Register tools based on configuration
            // For now, we register all tools by default
            use crate::core::tools::{ReadFileTool, RunShellCommandTool, WriteFileTool};
            tool_registry.add_tool(Box::new(WriteFileTool));
            tool_registry.add_tool(Box::new(ReadFileTool));
            tool_registry.add_tool(Box::new(RunShellCommandTool));
            self.tool_registry = Some(tool_registry);
        }
        Ok(self.tool_registry.as_mut().unwrap())
    }

    /// Build the session manager
    #[allow(dead_code)]
    pub fn build_session_manager(&mut self) -> Result<&mut SessionManager> {
        if self.session_manager.is_none() {
            // SessionManager is a unit struct with only static methods
            self.session_manager = Some(SessionManager);
        }
        Ok(self.session_manager.as_mut().unwrap())
    }

    /// Build the orchestrator
    pub fn build_orchestrator(
        &mut self,
        orchestrator_tx: mpsc::Sender<AppEvent>,
        orchestrator_rx: mpsc::Receiver<AppEvent>,
    ) -> Result<Orchestrator> {
        // Store configuration values to avoid borrowing issues
        let session_name = self.config.session.clone();
        let no_stream = self.config.no_stream;
        let agent_name = self.config.agent.name.clone();
        let agent_model = self.config.agent.model.clone();

        // Build dependencies (we call these to ensure they're initialized)
        let _agent = self.build_agent()?;
        let _tool_registry = self.build_tool_registry()?;

        // Create new instance for the orchestrator with the same configuration
        let agent_instance = Agent::new(&agent_name, &agent_model);

        // Create a new tool registry with the same tools
        let mut new_tool_registry = ToolRegistry::new();
        // Register the same tools
        use crate::core::tools::{ReadFileTool, RunShellCommandTool, WriteFileTool};
        new_tool_registry.add_tool(Box::new(WriteFileTool));
        new_tool_registry.add_tool(Box::new(ReadFileTool));
        new_tool_registry.add_tool(Box::new(RunShellCommandTool));

        Ok(Orchestrator::new(
            agent_instance,
            new_tool_registry,
            session_name,
            no_stream,
            orchestrator_tx,
            orchestrator_rx,
        ))
    }
}
