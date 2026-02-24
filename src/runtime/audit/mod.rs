//! Persistent audit trail for agent actions.
//!
//! Provides an append-only log of all agent actions, guardrail violations,
//! escalations, and workflow events. Supports JSON-lines output for SOC2 exports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter for `generate_id()` to ensure uniqueness within a process
/// even when two IDs are generated within the same nanosecond.
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn default_true() -> bool {
    true
}

/// Returns `true` if `*v` is `true`.
///
/// Used as `skip_serializing_if` predicate for `is_clock_reliable` so that
/// the field is omitted from serialized JSON when the clock is reliable (the
/// common case). Combined with `default = "default_true"`, absent fields
/// round-trip correctly for both old and new entries.
///
/// `bool` is `Copy`, but serde's `skip_serializing_if` always passes `&T`,
/// so the `&bool` argument is unavoidable here.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_true(v: &bool) -> bool {
    *v
}

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
    /// Whether the system clock was post-epoch when this entry's ID was
    /// generated. `false` means `duration_since(UNIX_EPOCH)` returned `Err`
    /// (clock before epoch — e.g. RTC not set, certain CI containers). The
    /// hex timestamp prefix in `id` will be `0` rather than a real timestamp,
    /// so compliance consumers must not rely on it for time ordering.
    ///
    /// Deserializes as `true` when the field is absent (entries produced before
    /// this field was added are assumed reliable). Omitted from serialized JSON
    /// when `true` (the common case) to keep audit lines compact; always
    /// written when `false` so compliance consumers can detect the anomaly.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub is_clock_reliable: bool,
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
///
/// Writes are serialized via an internal `Mutex<()>` so concurrent calls from
/// parallel workflow steps produce well-formed JSONL with no interleaved lines.
pub struct AuditLog {
    path: PathBuf,
    /// Guards `append` so concurrent parallel-step writes do not interleave.
    write_lock: Mutex<()>,
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
    ///
    /// Creates parent directories if needed and verifies that the probe
    /// directory is writable **without** creating the target file. The target
    /// file is created lazily on the first `append` call. This prevents
    /// compliance tools from seeing a zero-byte audit file and misinterpreting
    /// it as "logging is active but nothing was approved" when the workflow
    /// never executed (e.g. because `.rein` validation failed after
    /// `AuditLog::new`).
    ///
    /// **Bare-filename fallback:** when `path` is a bare filename with no
    /// directory component (e.g. `"audit.jsonl"`), `parent()` returns an
    /// empty path. In that case the probe falls back to `current_dir()` so
    /// the writability check is always performed against a real directory.
    ///
    /// The writability probe uses a temporary file in the same directory.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, AuditError> {
        let path = path.into();
        // Resolve the probe directory. `parent()` returns `Some("")` for a
        // bare filename ("audit.jsonl") which is not a usable directory path.
        // Fall back to cwd explicitly rather than silently skipping the probe.
        let probe_dir = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => {
                fs::create_dir_all(p)?;
                p.to_path_buf()
            }
            _ => std::env::current_dir()?,
        };
        // Probe writability via a temp file in the probe directory without
        // creating or touching the target file. The temp file is created and
        // immediately removed so it leaves no side effect.
        // Use generate_id() suffix to avoid collisions when multiple processes
        // or parallel test threads create audit logs in the same directory.
        let (probe_id, _) = generate_id();
        let probe = probe_dir.join(format!(".rein-audit-probe-{probe_id}"));
        fs::File::create(&probe)?;
        // Cleanup is best-effort: writability is already confirmed by the
        // successful create above. If remove_file fails (e.g. the file was
        // deleted by a concurrent actor), propagating the error would reject
        // a writable directory, which is misleading. The probe file is
        // self-documenting by its name prefix (.rein-audit-probe-*).
        let _ = fs::remove_file(&probe);
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }

    /// Append an entry to the audit log (append-only).
    ///
    /// Acquires `write_lock` before opening the file so concurrent calls from
    /// parallel workflow steps do not interleave partial JSONL lines.
    pub fn append(&self, entry: &AuditEntry) -> Result<(), AuditError> {
        // Recover from lock poison: the guard holds `()` so a poisoned lock
        // carries no invalid state. Panicking here would abort the approval
        // flow and violate the fail-open-on-write contract.
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
}

/// Generate a unique audit event ID.
///
/// Returns `(id, is_clock_reliable)` where `is_clock_reliable` is `false`
/// when the system clock is set before the Unix epoch (e.g. RTC not set,
/// certain CI containers). In that case the hex timestamp prefix in `id`
/// will be `0` rather than a real timestamp; IDs remain unique because the
/// atomic sequence counter still increments.
///
/// Compliance consumers that parse the prefix for time ordering should
/// treat `audit-0-N` IDs (or any entry with `is_clock_reliable: false`)
/// as having unknown wall-clock time.
///
/// This is a module-level function rather than an `AuditLog` method because
/// it has no dependency on `AuditLog` state — it reads only the process-global
/// `ID_COUNTER` and the system clock.
pub(crate) fn generate_id() -> (String, bool) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let clock_result = SystemTime::now().duration_since(UNIX_EPOCH);
    let is_clock_reliable = clock_result.is_ok();
    let nanos = clock_result.unwrap_or_default().as_nanos();
    let seq = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    (format!("audit-{nanos:x}-{seq}"), is_clock_reliable)
}

/// Convenience builder for audit entries.
pub fn entry(kind: AuditKind, description: impl Into<String>) -> AuditEntry {
    let (id, is_clock_reliable) = generate_id();
    AuditEntry {
        id,
        timestamp: Utc::now(),
        kind,
        workflow: None,
        agent: None,
        step: None,
        description: description.into(),
        metadata: serde_json::Value::Null,
        is_clock_reliable,
    }
}

#[cfg(test)]
mod tests;
