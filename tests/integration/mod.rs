//! Integration tests module.

mod test_cli;

#[cfg(test)]
mod mcp {
    mod test_config_integration;
}

#[cfg(test)]
mod core {
    mod test_mocked_external_deps;
    mod test_orchestrator_agent_interactions;
    mod test_tool_approval_workflow;
    mod test_tool_interactions;
    mod tool_permissions;
}
