use std::borrow::Cow;
use std::sync::Arc;

use crate::ast::{ApprovalDef, ApprovalKind};
use crate::runtime::audit::{self, AuditKind, AuditLog};

#[cfg(test)]
mod tests;

/// Maximum byte length of `agent_output` forwarded in approval payloads and
/// recorded in audit entries.
///
/// Outputs longer than this limit are truncated and the `agent_output_truncated`
/// field in audit metadata is set to `true`. All handlers and tests reference
/// this constant so the limit has a single source of truth.
pub const AGENT_OUTPUT_PREVIEW_LIMIT: usize = 512;

/// Suffix appended to `agent_output` when truncation occurs.
///
/// The `agent_output_truncated` bool is the machine-readable signal; this
/// marker is the human-readable companion. Tests should compute expected
/// maximum lengths using `AGENT_OUTPUT_PREVIEW_LIMIT + TRUNCATION_MARKER.len()`
/// rather than hardcoding the combined value.
pub const TRUNCATION_MARKER: &str = "… (truncated)";

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
        // Cap at AGENT_OUTPUT_PREVIEW_LIMIT bytes — consistent with webhook/Slack/audit (#514).
        let preview = truncate_agent_output(agent_output);
        for line in preview.lines() {
            eprintln!("║  {line}");
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

/// Truncate `output` to [`AGENT_OUTPUT_PREVIEW_LIMIT`] bytes, appending
/// [`TRUNCATION_MARKER`] when a cut is made.
///
/// Returns a borrowed `Cow` (no allocation) when the output is within the limit;
/// returns an owned `Cow` (one allocation) only when truncation is required.
/// Callers can detect truncation via `matches!(result, Cow::Owned(_))`.
fn truncate_agent_output(output: &str) -> Cow<'_, str> {
    if output.len() > AGENT_OUTPUT_PREVIEW_LIMIT {
        let cut = output.floor_char_boundary(AGENT_OUTPUT_PREVIEW_LIMIT);
        Cow::Owned(format!("{}{}", &output[..cut], TRUNCATION_MARKER))
    } else {
        Cow::Borrowed(output)
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
        // Cap agent_output to match AuditingApprovalHandler's preview limit so
        // webhook payloads are consistent with audit records (#500).
        let output_preview = truncate_agent_output(agent_output);
        let payload = serde_json::json!({
            "step": step_name,
            "channel": approval.channel,
            "agent_output": &*output_preview,
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
        // Cap agent_output to match AuditingApprovalHandler's preview limit so
        // Slack messages are consistent with audit records (#500).
        let output_preview = truncate_agent_output(agent_output);
        let timeout_str = approval.timeout.as_deref().unwrap_or("no timeout");
        let text = format!(
            "Approval required: step '{step_name}'\nTimeout: {timeout_str}\n\nAgent output:\n{output_preview}"
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

/// Wraps any `ApprovalHandler` (as an `Arc<dyn ApprovalHandler>`) and emits
/// `ApprovalRequested` / `ApprovalResolved` audit entries before and after
/// each approval decision.
///
/// Accepting `Arc<dyn ApprovalHandler>` directly (rather than a generic `H:
/// ApprovalHandler`) eliminates the blanket impl on `Arc<dyn ApprovalHandler>`
/// that would have been required to bridge the generic and the trait-object
/// injection site in `run_step`. Callers wrap concrete handlers with
/// `Arc::new(handler)` before passing them here.
///
/// This is the canonical way to add audit trails to approval flows — callers
/// construct the appropriate inner handler and wrap it here.
pub struct AuditingApprovalHandler {
    inner: Arc<dyn ApprovalHandler>,
    log: Arc<AuditLog>,
    workflow_name: Option<String>,
    agent_name: Option<String>,
}

impl AuditingApprovalHandler {
    #[must_use]
    pub fn new(inner: Arc<dyn ApprovalHandler>, log: Arc<AuditLog>) -> Self {
        Self {
            inner,
            log,
            workflow_name: None,
            agent_name: None,
        }
    }

    /// Attach a workflow name to every audit entry emitted by this handler.
    ///
    /// Empty strings are silently ignored; the workflow field will remain `None`.
    /// This guard is self-enforcing at the method boundary: callers cannot produce
    /// `workflow: Some("")` in audit records regardless of what they pass in.
    #[must_use]
    pub fn with_workflow(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if !name.is_empty() {
            self.workflow_name = Some(name);
        }
        self
    }

    /// Attach an agent name to every audit entry emitted by this handler.
    ///
    /// Empty strings are silently ignored; the agent field will remain `None`.
    /// This guard is self-enforcing at the method boundary: callers cannot produce
    /// `agent: Some("")` in audit records regardless of what they pass in.
    #[must_use]
    pub fn with_agent(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if !name.is_empty() {
            self.agent_name = Some(name);
        }
        self
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for AuditingApprovalHandler {
    async fn request_approval(
        &self,
        step_name: &str,
        agent_output: &str,
        approval: &ApprovalDef,
    ) -> ApprovalStatus {
        // Emit ApprovalRequested before delegating.
        let mut requested = audit::entry(
            AuditKind::ApprovalRequested,
            format!("Approval requested for step '{step_name}'"),
        );
        requested.step = Some(step_name.to_string());
        requested.workflow = self.workflow_name.clone();
        requested.agent = self.agent_name.clone();
        // Truncate agent_output to AGENT_OUTPUT_PREVIEW_LIMIT bytes to avoid unbounded audit
        // log growth. floor_char_boundary (inside truncate_agent_output) ensures the slice
        // ends on a valid UTF-8 boundary even when the input contains multibyte characters.
        // INVARIANT: truncate_agent_output returns Cow::Owned only when a cut was made.
        // If that function is ever changed to return Owned for other reasons (e.g. normalization),
        // this bool would silently lie to compliance consumers — update the derivation then.
        let output_preview = truncate_agent_output(agent_output);
        let truncated = matches!(output_preview, Cow::Owned(_));
        let mut req_meta = serde_json::json!({
            "channel": approval.channel,
            "agent_output": &*output_preview,
            "agent_output_truncated": truncated,
        });
        if let Some(ref t) = approval.timeout {
            req_meta["timeout"] = serde_json::Value::String(t.clone());
        }
        requested.metadata = req_meta;
        if let Err(e) = self.log.append(&requested) {
            // TODO(#377): replace with tracing::warn! once the `tracing` crate
            // is added to Cargo.toml.
            eprintln!(
                "rein[audit]: warning: could not write ApprovalRequested entry \
                 (step='{}', workflow='{}'): {e}",
                step_name,
                self.workflow_name.as_deref().unwrap_or("<none>")
            );
        }
        // Start the clock after writing ApprovalRequested so elapsed_ms
        // captures only the gate-open time: from the moment the approval is
        // visible to the handler until it returns its decision. Including the
        // I/O write time would misrepresent the approval latency for compliance
        // consumers that compare elapsed_ms against SLA thresholds.
        let start = std::time::Instant::now();
        let status = self
            .inner
            .request_approval(step_name, agent_output, approval)
            .await;

        // Saturate at u64::MAX rather than failing; a run long enough to
        // overflow (~585 million years) is not realistic in practice.
        #[allow(clippy::cast_possible_truncation)]
        let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let decision = match &status {
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected { .. } => "rejected",
            // `TimedOut` maps to `WorkflowError::ApprovalTimedOut`; `Pending` now maps
            // to `WorkflowError::ApprovalPending` (split in #419). Both are recorded as
            // "timed_out" here for compliance-field stability — the distinction is
            // preserved in the `original_status` field (see below) so operators can
            // distinguish a genuine timeout from a deferred async decision without
            // affecting compliance parsers. If a resume path for Pending is added in
            // future, this mapping must be revisited.
            ApprovalStatus::TimedOut | ApprovalStatus::Pending => "timed_out",
        };
        // Record the raw handler status separately from the compliance-stable
        // `decision` field. This preserves diagnostic fidelity: an operator can
        // distinguish a genuine timeout from a `Pending` return (e.g. a
        // misconfigured async handler) without affecting compliance parsers.
        let original_status = match &status {
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected { .. } => "rejected",
            ApprovalStatus::TimedOut => "timed_out",
            ApprovalStatus::Pending => "pending",
        };
        // Capture the rejection reason if present so the audit record can
        // reconstruct _why_ an approval was rejected (not just that it was).
        let rejection_reason: Option<String> = match &status {
            ApprovalStatus::Rejected { reason } => Some(reason.clone()),
            _ => None,
        };

        // Emit ApprovalResolved after delegating.
        let mut resolved = audit::entry(
            AuditKind::ApprovalResolved,
            format!("Approval resolved for step '{step_name}': {decision}"),
        );
        resolved.step = Some(step_name.to_string());
        resolved.workflow.clone_from(&self.workflow_name);
        resolved.agent.clone_from(&self.agent_name);
        let mut meta = serde_json::json!({
            "channel": approval.channel,
            "decision": decision,
            "elapsed_ms": elapsed_ms,
        });
        // Only include "reason" for rejected decisions; omitting the key for
        // approved/timed-out outcomes avoids a noisy `null` in audit records.
        if let Some(r) = rejection_reason {
            meta["reason"] = serde_json::Value::String(r);
        }
        // Include "original_status" only when it diverges from "decision" —
        // i.e., when the handler returned Pending but the compliance field
        // shows "timed_out". This allows operators to diagnose handler
        // misconfigurations without disrupting compliance consumers.
        if original_status != decision {
            meta["original_status"] = serde_json::Value::String(original_status.to_string());
        }
        resolved.metadata = meta;
        if let Err(e) = self.log.append(&resolved) {
            // TODO(#377): replace with tracing::warn! once the `tracing` crate
            // is added to Cargo.toml.
            eprintln!(
                "rein[audit]: warning: could not write ApprovalResolved entry \
                 (step='{}', workflow='{}', decision='{decision}'): {e}",
                step_name,
                self.workflow_name.as_deref().unwrap_or("<none>")
            );
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
