use serde::{Deserialize, Serialize};

use super::Span;

/// An `approve` or `collaborate` human-in-the-loop block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalDef {
    /// Kind of approval (approve or collaborate).
    pub kind: ApprovalKind,
    /// Channel for the approval (e.g. "slack", "dashboard").
    pub channel: String,
    /// Destination within the channel (e.g. "#approvals").
    pub destination: String,
    /// Optional timeout (e.g. "4h", "30m").
    pub timeout: Option<String>,
    /// Collaboration mode (only for collaborate kind).
    pub mode: Option<CollaborationMode>,
    pub span: Span,
}

/// Whether this is an approval gate or a collaboration session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    /// Simple approve/reject gate.
    Approve,
    /// Human edits, suggests, or reviews agent output.
    Collaborate,
}

/// Mode for human collaboration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationMode {
    /// Human can directly edit agent output.
    Edit,
    /// Human suggests changes, agent decides.
    Suggest,
    /// Human reviews and approves/rejects.
    Review,
}
