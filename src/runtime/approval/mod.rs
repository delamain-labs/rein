use std::sync::Arc;

use crate::ast::{ApprovalDef, ApprovalKind};
use crate::runtime::audit::{self, AuditKind, AuditLog};

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

/// A webhook-based approval handler.
///
/// POSTs a JSON payload (including the agent's output) to the configured URL.
/// On 2xx the step is approved. On non-2xx or network error the step is
/// rejected (fail-closed) — an unreachable or erroring approval endpoint must
/// never silently grant access.
pub struct WebhookApprovalHandler {
    url: String,
    client: reqwest::Client,
}

impl WebhookApprovalHandler {
    #[must_use]
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for WebhookApprovalHandler {
    async fn request_approval(
        &self,
        step_name: &str,
        agent_output: &str,
        approval: &ApprovalDef,
    ) -> ApprovalStatus {
        let payload = serde_json::json!({
            "step": step_name,
            "channel": approval.channel,
            "agent_output": agent_output,
        });

        match self.client.post(&self.url).json(&payload).send().await {
            Ok(resp) if resp.status().is_success() => {
                // 2xx: webhook accepted the notification; step is synchronously approved (v1).
                // Interactive async callbacks (e.g. Block Kit) are deferred to v2.
                eprintln!("[webhook] Approval notification sent for step '{step_name}'");
                ApprovalStatus::Approved
            }
            Ok(resp) => {
                let status = resp.status();
                eprintln!(
                    "[webhook] Approval endpoint returned {status} for step '{step_name}': rejecting"
                );
                ApprovalStatus::Rejected {
                    reason: format!("webhook returned {status}"),
                }
            }
            Err(e) => {
                eprintln!(
                    "[webhook] Failed to reach approval endpoint for step '{step_name}': {e} — rejecting"
                );
                ApprovalStatus::Rejected {
                    reason: format!("webhook unreachable: {e}"),
                }
            }
        }
    }
}

/// A Slack-based approval handler.
///
/// POSTs a formatted message (including the agent's output) to the Slack
/// incoming webhook URL, then auto-approves the step. Full interactive Slack
/// approval (Block Kit buttons with callback handling) is deferred to a
/// follow-up.
pub struct SlackApprovalHandler {
    webhook_url: String,
    client: reqwest::Client,
}

impl SlackApprovalHandler {
    #[must_use]
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for SlackApprovalHandler {
    async fn request_approval(
        &self,
        step_name: &str,
        agent_output: &str,
        approval: &ApprovalDef,
    ) -> ApprovalStatus {
        let timeout_str = approval.timeout.as_deref().unwrap_or("no timeout");
        let text = format!(
            "Approval required: step '{step_name}'\nTimeout: {timeout_str}\n\nAgent output:\n{agent_output}"
        );
        let payload = serde_json::json!({ "text": text });

        match self
            .client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                eprintln!("[slack] Approval notification sent for step '{step_name}'");
            }
            Ok(resp) => {
                eprintln!(
                    "[slack] Slack endpoint returned {}: auto-approving step '{step_name}'",
                    resp.status()
                );
            }
            Err(e) => {
                eprintln!("[slack] Failed to send Slack notification for step '{step_name}': {e}");
                eprintln!("[slack] Auto-approving to avoid blocking workflow");
            }
        }

        // MVP: notify-and-auto-approve. Interactive callbacks are v2.
        ApprovalStatus::Approved
    }
}

/// Wraps any `ApprovalHandler` and emits `ApprovalRequested` / `ApprovalResolved`
/// audit entries before and after each approval decision.
///
/// This is the canonical way to add audit trails to approval flows — callers
/// construct the appropriate inner handler and wrap it here.
pub struct AuditingApprovalHandler<H> {
    inner: H,
    log: Arc<AuditLog>,
}

impl<H> AuditingApprovalHandler<H> {
    pub fn new(inner: H, log: Arc<AuditLog>) -> Self {
        Self { inner, log }
    }
}

#[async_trait::async_trait]
impl<H: ApprovalHandler> ApprovalHandler for AuditingApprovalHandler<H> {
    async fn request_approval(
        &self,
        step_name: &str,
        agent_output: &str,
        approval: &ApprovalDef,
    ) -> ApprovalStatus {
        let start = std::time::Instant::now();

        // Emit ApprovalRequested before delegating.
        let mut requested = audit::entry(
            AuditKind::ApprovalRequested,
            format!("Approval requested for step '{step_name}'"),
        );
        requested.step = Some(step_name.to_string());
        requested.metadata = serde_json::json!({
            "channel": approval.channel,
            "timeout": approval.timeout,
        });
        if let Err(e) = self.log.append(&requested) {
            // TODO: replace with tracing::warn! once the `tracing` crate is
            // available.
            eprintln!("rein[audit]: warning: could not write ApprovalRequested entry: {e}");
        }

        let status = self
            .inner
            .request_approval(step_name, agent_output, approval)
            .await;

        // Saturate at u64::MAX rather than failing; a run long enough to
        // overflow (~585 million years) is not realistic in practice.
        // TODO: replace eprintln! with tracing::warn! once the `tracing` crate
        // is added to Cargo.toml.
        #[allow(clippy::cast_possible_truncation)]
        let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let decision = match &status {
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected { .. } => "rejected",
            ApprovalStatus::TimedOut => "timed_out",
            // `Pending` is returned by the inner handler if it defers to an
            // async polling mechanism. In practice the CLI/webhook handlers
            // always resolve synchronously, so this arm is unlikely to fire
            // in production. It is kept here for completeness.
            ApprovalStatus::Pending => "pending",
        };

        // Emit ApprovalResolved after delegating.
        let mut resolved = audit::entry(
            AuditKind::ApprovalResolved,
            format!("Approval resolved for step '{step_name}': {decision}"),
        );
        resolved.step = Some(step_name.to_string());
        resolved.metadata = serde_json::json!({
            "channel": approval.channel,
            "decision": decision,
            "elapsed_ms": elapsed_ms,
        });
        if let Err(e) = self.log.append(&resolved) {
            // TODO: replace with tracing::warn! once the `tracing` crate is
            // available.
            eprintln!("rein[audit]: warning: could not write ApprovalResolved entry: {e}");
        }

        status
    }
}

/// Select an `ApprovalHandler` based on the channel type in the `ApprovalDef`.
///
/// - `"webhook"` → `WebhookApprovalHandler` (POST to `destination` URL)
/// - `"slack"` → `SlackApprovalHandler` (POST to `destination` Slack webhook URL)
/// - `"cli"` → `CliApprovalHandler` (interactive stdin prompt)
/// - anything else → warning + `CliApprovalHandler` fallback
#[must_use]
pub fn resolve_approval_handler(approval: &ApprovalDef) -> Box<dyn ApprovalHandler> {
    match approval.channel.as_str() {
        "webhook" => Box::new(WebhookApprovalHandler::new(approval.destination.clone())),
        "slack" => Box::new(SlackApprovalHandler::new(approval.destination.clone())),
        "cli" => Box::new(CliApprovalHandler),
        "" => {
            eprintln!("warn: approval channel is empty, falling back to CLI prompt");
            Box::new(CliApprovalHandler)
        }
        other => {
            eprintln!("warn: unknown approval channel '{other}', falling back to CLI prompt");
            Box::new(CliApprovalHandler)
        }
    }
}
