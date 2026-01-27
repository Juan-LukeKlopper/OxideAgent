//! Dependency injection container for the OxideAgent system.
//!
//! This module implements a service container for managing dependencies
//! between components in the application.

use crate::config::OxideConfig;
use crate::core::mcp_manager::McpManager;
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
    tool_registry: Option<ToolRegistry>,
    #[allow(dead_code)]
    session_manager: Option<SessionManager>,
}

impl Container {
    /// Create a new container with the given configuration
    pub fn new(config: OxideConfig) -> Self {
        Self {
            config: Arc::new(config),
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

    /// Build the tool registry
    pub async fn build_tool_registry(&mut self) -> Result<&mut ToolRegistry> {
        if self.tool_registry.is_none() {
            let mut tool_registry = ToolRegistry::new();
            // Register tools based on configuration
            // For now, we register all tools by default
            use crate::core::tools::{ReadFileTool, RunShellCommandTool, WriteFileTool};
            tool_registry.add_tool(Box::new(WriteFileTool));
            tool_registry.add_tool(Box::new(ReadFileTool));
            tool_registry.add_tool(Box::new(RunShellCommandTool));

            // Log MCP configuration if present
            if !self.config.mcp.tools.is_empty() {
                use tracing::info;
                info!(
                    "MCP configuration found with {} tool(s)",
                    self.config.mcp.tools.len()
                );
                for tool in &self.config.mcp.tools {
                    info!("  - MCP Tool: {} (command: {})", tool.name, tool.command);
                }
            } else {
                use tracing::debug;
                debug!("No MCP tools configured");
            }

            let mut mcp_manager = McpManager::new(tool_registry);
            mcp_manager.launch_servers(&self.config.mcp.tools).await?;
            mcp_manager.launch_remote_server(&self.config.mcp).await?;
            self.tool_registry = Some(mcp_manager.into_tool_registry());

            // Log the final tools in the registry
            let final_tools = self.tool_registry.as_ref().unwrap().definitions();
            use tracing::info;
            info!("Final tool registry contains {} tools:", final_tools.len());
            for tool in &final_tools {
                info!(
                    "  - Registered tool: {} - {}",
                    tool.function.name,
                    tool.truncated_description()
                );
            }
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
    pub async fn build_orchestrator(
        &mut self,
        orchestrator_tx: mpsc::Sender<AppEvent>,
        orchestrator_rx: mpsc::Receiver<AppEvent>,
    ) -> Result<Orchestrator> {
        // Store configuration values to avoid borrowing issues
        let session_name = self.config.session.clone();
        let no_stream = self.config.no_stream;
        let system_prompt = self.config.agent.system_prompt.clone();
        let model = self.config.agent.model.clone();
        let llm_config = self.config.llm.clone();

        // Build dependencies (we call these to ensure they're initialized)
        let tool_registry = self.build_tool_registry().await?;

        Ok(Orchestrator::new(
            &system_prompt,
            tool_registry.clone_registry(),
            session_name,
            no_stream,
            orchestrator_tx,
            orchestrator_rx,
            model,
            llm_config,
        ))
    }
}
