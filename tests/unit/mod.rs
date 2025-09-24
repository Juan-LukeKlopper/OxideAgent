//! Unit tests module.

mod test_config;
mod test_utils;

#[cfg(test)]
mod core {
    mod test_agents;
    mod test_container;
    mod test_events;
    mod test_orchestrator;
    mod test_session;
    mod test_tools;
    mod test_tool_permissions;
    mod test_session_tool_permissions;
}

#[cfg(test)]
mod mcp {
    mod test_config;
}
