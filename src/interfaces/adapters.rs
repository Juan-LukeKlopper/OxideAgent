//! Shared adapter utilities for interface transports.

use crate::types::{AppEvent, ToolApprovalResponse};

/// Normalized approval actions exposed by interface transports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ApprovalAction {
    ApproveOnce,
    ApproveAlwaysGlobal,
    ApproveAlwaysSession,
    Deny,
}

impl From<ApprovalAction> for ToolApprovalResponse {
    fn from(value: ApprovalAction) -> Self {
        match value {
            ApprovalAction::ApproveOnce => ToolApprovalResponse::Allow,
            ApprovalAction::ApproveAlwaysGlobal => ToolApprovalResponse::AlwaysAllow,
            ApprovalAction::ApproveAlwaysSession => ToolApprovalResponse::AlwaysAllowSession,
            ApprovalAction::Deny => ToolApprovalResponse::Deny,
        }
    }
}

/// Tracks transport-facing approval state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ApprovalFlowState {
    Idle,
    AwaitingDecision,
    Completed,
}

/// Small state machine wrapper for approval workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ApprovalFlow {
    state: ApprovalFlowState,
}

impl Default for ApprovalFlow {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ApprovalFlow {
    pub const fn new() -> Self {
        Self {
            state: ApprovalFlowState::Idle,
        }
    }

    pub fn request_approval(&mut self) {
        self.state = ApprovalFlowState::AwaitingDecision;
    }

    pub fn decide(&mut self, action: ApprovalAction) -> Option<AppEvent> {
        if self.state != ApprovalFlowState::AwaitingDecision {
            return None;
        }

        self.state = ApprovalFlowState::Completed;
        Some(AppEvent::ToolApproval(action.into()))
    }

    pub const fn state(&self) -> ApprovalFlowState {
        self.state
    }

    pub fn reset(&mut self) {
        self.state = ApprovalFlowState::Idle;
    }
}

/// Chunk buffer for interfaces that cannot stream every token immediately.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct MessageChunkBuffer {
    chunks: Vec<String>,
}

#[allow(dead_code)]
impl MessageChunkBuffer {
    pub fn push_chunk(&mut self, chunk: impl Into<String>) {
        self.chunks.push(chunk.into());
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn flush(&mut self) -> Option<String> {
        if self.chunks.is_empty() {
            return None;
        }

        let result = self.chunks.join("");
        self.chunks.clear();
        Some(result)
    }
}

/// Normalize an error for user-facing rendering.
#[allow(dead_code)]
pub fn normalize_error(error: &anyhow::Error) -> String {
    format!("Interface error: {error:#}")
}

#[cfg(test)]
mod tests {
    use super::{ApprovalAction, ApprovalFlow, ApprovalFlowState, MessageChunkBuffer};
    use crate::types::{AppEvent, ToolApprovalResponse};

    #[test]
    fn approval_flow_requires_request_before_decision() {
        let mut flow = ApprovalFlow::new();
        assert_eq!(flow.state(), ApprovalFlowState::Idle);
        assert!(flow.decide(ApprovalAction::ApproveOnce).is_none());

        flow.request_approval();
        let event = flow.decide(ApprovalAction::ApproveAlwaysSession);
        assert!(matches!(
            event,
            Some(AppEvent::ToolApproval(
                ToolApprovalResponse::AlwaysAllowSession
            ))
        ));
        assert_eq!(flow.state(), ApprovalFlowState::Completed);
    }

    #[test]
    fn message_chunk_buffer_flushes_all_chunks() {
        let mut buffer = MessageChunkBuffer::default();
        assert!(buffer.is_empty());

        buffer.push_chunk("hel");
        buffer.push_chunk("lo");
        assert_eq!(buffer.flush().as_deref(), Some("hello"));
        assert!(buffer.flush().is_none());
    }
}
