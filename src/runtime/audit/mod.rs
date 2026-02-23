//! Persistent audit trail for agent actions.
//!
//! Provides an append-only log of all agent actions, guardrail violations,
//! escalations, and workflow events. Supports JSON-lines output for SOC2 exports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry {
    /// Unique event ID.
    pub id: String,
    /// ISO-8601 timestamp.
    pub timestamp: DateTime<Utc>,
    /// Event kind.
    pub kind: AuditKind,
    /// Workflow name (if applicable).
    pub workflow: Option<String>,
    /// Agent name (if applicable).
    pub agent: Option<String>,
    /// Step or stage name.
    pub step: Option<String>,
    /// Human-readable description.
    pub description: String,
    /// Additional structured metadata.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub metadata: serde_json::Value,
}

/// Categories of auditable events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    /// Agent performed a tool call.
    ToolCall,
    /// Agent produced a response.
    AgentResponse,
    /// Guardrail was violated.
    GuardrailViolation,
    /// Escalation triggered.
    Escalation,
    /// Workflow started.
    WorkflowStart,
    /// Workflow completed.
    WorkflowComplete,
    /// Workflow failed.
    WorkflowFailed,
    /// Step started.
    StepStart,
    /// Step completed.
    StepComplete,
    /// Tool call denied by permissions.
    ToolDenied,
    /// Budget limit reached.
    BudgetExceeded,
    /// Approval was requested for a workflow step.
    ApprovalRequested,
    /// Approval decision was recorded for a workflow step.
    ApprovalResolved,
    /// Custom event.
    Custom(String),
}

/// Persistent audit log backed by JSON-lines files.
pub struct AuditLog {
    path: PathBuf,
}

/// Error type for audit operations.
#[derive(Debug)]
pub enum AuditError {
    Io(io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "audit I/O error: {e}"),
            Self::Serde(e) => write!(f, "audit serialization error: {e}"),
        }
    }
}

impl std::error::Error for AuditError {}

impl From<io::Error> for AuditError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for AuditError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

impl AuditLog {
    /// Create a new audit log at the given path.
    /// Creates parent directories if needed.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, AuditError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    /// Append an entry to the audit log (append-only).
    pub fn append(&self, entry: &AuditEntry) -> Result<(), AuditError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    /// Read all entries from the audit log.
    pub fn read_all(&self) -> Result<Vec<AuditEntry>, AuditError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            entries.push(serde_json::from_str(&line)?);
        }
        Ok(entries)
    }

    /// Query entries by kind.
    pub fn query_by_kind(&self, kind: &AuditKind) -> Result<Vec<AuditEntry>, AuditError> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|e| &e.kind == kind)
            .collect())
    }

    /// Query entries by workflow name.
    pub fn query_by_workflow(&self, workflow: &str) -> Result<Vec<AuditEntry>, AuditError> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|e| e.workflow.as_deref() == Some(workflow))
            .collect())
    }

    /// Return the path to the audit log file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Generate a unique event ID.
    pub fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("audit-{nanos:x}")
    }
}

/// Convenience builder for audit entries.
pub fn entry(kind: AuditKind, description: impl Into<String>) -> AuditEntry {
    AuditEntry {
        id: AuditLog::generate_id(),
        timestamp: Utc::now(),
        kind,
        workflow: None,
        agent: None,
        step: None,
        description: description.into(),
        metadata: serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests;
