//! Shared interface capability definitions.

/// Declares which interaction patterns an interface supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct InterfaceCapabilities {
    pub streaming_output: bool,
    pub interactive_tool_approval: bool,
    pub agent_switching: bool,
    pub session_switching: bool,
    pub rich_message_rendering: bool,
}

#[allow(dead_code)]
impl InterfaceCapabilities {
    pub const fn tui() -> Self {
        Self {
            streaming_output: true,
            interactive_tool_approval: true,
            agent_switching: true,
            session_switching: true,
            rich_message_rendering: true,
        }
    }

    pub const fn web() -> Self {
        Self {
            streaming_output: true,
            interactive_tool_approval: true,
            agent_switching: true,
            session_switching: true,
            rich_message_rendering: true,
        }
    }

    pub const fn telegram() -> Self {
        Self {
            streaming_output: false,
            interactive_tool_approval: true,
            agent_switching: true,
            session_switching: true,
            rich_message_rendering: false,
        }
    }

    pub const fn discord() -> Self {
        Self {
            streaming_output: true,
            interactive_tool_approval: true,
            agent_switching: true,
            session_switching: true,
            rich_message_rendering: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InterfaceCapabilities;

    #[test]
    fn capability_profiles_are_sane() {
        let tui = InterfaceCapabilities::tui();
        assert!(tui.streaming_output);
        assert!(tui.interactive_tool_approval);

        let telegram = InterfaceCapabilities::telegram();
        assert!(!telegram.streaming_output);
        assert!(!telegram.rich_message_rendering);

        let web = InterfaceCapabilities::web();
        let discord = InterfaceCapabilities::discord();
        assert!(web.agent_switching && discord.agent_switching);
    }
}
