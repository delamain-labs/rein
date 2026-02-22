use crate::ast::{ApprovalDef, ApprovalKind};

#[cfg(test)]
mod tests;

/// The result of an approval request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// Approved by a human reviewer.
    Approved,
    /// Rejected by a human reviewer.
    Rejected { reason: String },
    /// Timed out waiting for approval.
    TimedOut,
    /// Approval is pending (async flow).
    Pending,
}

/// A callback trait for handling approval requests.
/// Implementations can be interactive (CLI prompt), async (webhook), or mock.
#[async_trait::async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// Request approval for a step's output.
    /// Returns the approval status.
    async fn request_approval(
        &self,
        step_name: &str,
        agent_output: &str,
        approval: &ApprovalDef,
    ) -> ApprovalStatus;
}

/// A CLI-based approval handler that prompts the user interactively.
pub struct CliApprovalHandler;

#[async_trait::async_trait]
impl ApprovalHandler for CliApprovalHandler {
    async fn request_approval(
        &self,
        step_name: &str,
        agent_output: &str,
        approval: &ApprovalDef,
    ) -> ApprovalStatus {
        let kind_label = match approval.kind {
            ApprovalKind::Approve => "APPROVAL REQUIRED",
            ApprovalKind::Collaborate => "COLLABORATION REQUIRED",
        };

        eprintln!();
        eprintln!("╔══════════════════════════════════════════╗");
        eprintln!("║  🛑 {kind_label}");
        eprintln!("╠══════════════════════════════════════════╣");
        eprintln!("║  Step: {step_name}");
        eprintln!(
            "║  Channel: {} → {}",
            approval.channel, approval.destination
        );
        if let Some(ref timeout) = approval.timeout {
            eprintln!("║  Timeout: {timeout}");
        }
        eprintln!("╠══════════════════════════════════════════╣");
        eprintln!("║  Agent output:");
        for line in agent_output.lines().take(10) {
            eprintln!("║  {line}");
        }
        if agent_output.lines().count() > 10 {
            eprintln!("║  ... ({} more lines)", agent_output.lines().count() - 10);
        }
        eprintln!("╚══════════════════════════════════════════╝");
        eprintln!();

        eprint!("Approve? [y/n]: ");

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return ApprovalStatus::Rejected {
                reason: "Failed to read input".to_string(),
            };
        }

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => ApprovalStatus::Approved,
            _ => ApprovalStatus::Rejected {
                reason: "Human reviewer rejected".to_string(),
            },
        }
    }
}

/// An auto-approve handler for non-interactive environments (CI, testing).
pub struct AutoApproveHandler;

#[async_trait::async_trait]
impl ApprovalHandler for AutoApproveHandler {
    async fn request_approval(
        &self,
        step_name: &str,
        _agent_output: &str,
        _approval: &ApprovalDef,
    ) -> ApprovalStatus {
        eprintln!("[auto-approve] Step '{step_name}' auto-approved (non-interactive mode)");
        ApprovalStatus::Approved
    }
}

/// An auto-reject handler for testing rejection flows.
pub struct AutoRejectHandler {
    reason: String,
}

impl AutoRejectHandler {
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for AutoRejectHandler {
    async fn request_approval(
        &self,
        _step_name: &str,
        _agent_output: &str,
        _approval: &ApprovalDef,
    ) -> ApprovalStatus {
        ApprovalStatus::Rejected {
            reason: self.reason.clone(),
        }
    }
}

/// Parse a timeout string like "4h" or "30m" into seconds.
#[must_use]
pub fn parse_timeout(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(hours) = s.strip_suffix('h') {
        hours.parse::<u64>().ok().map(|h| h * 3600)
    } else if let Some(mins) = s.strip_suffix('m') {
        mins.parse::<u64>().ok().map(|m| m * 60)
    } else if let Some(secs) = s.strip_suffix('s') {
        secs.parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}
