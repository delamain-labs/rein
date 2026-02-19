use serde::{Deserialize, Serialize};
use std::path::Path;

/// Serializable snapshot of a workflow run's progress.
///
/// Written to disk as JSON after each stage completes so a crashed workflow
/// can resume from the last completed stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowState {
    /// Name of the workflow.
    pub workflow_name: String,
    /// Names of stages that have already completed (in execution order).
    pub completed_stages: Vec<CompletedStage>,
    /// The input that should be fed to the next stage.
    pub next_input: String,
}

/// A stage that completed successfully, stored for resume.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletedStage {
    pub stage_name: String,
    pub agent_name: String,
    pub output: String,
    pub cost_cents: u64,
    pub tokens: u64,
}

/// Errors from state persistence operations.
#[derive(Debug)]
pub enum PersistenceError {
    /// Failed to read or write the state file.
    Io(std::io::Error),
    /// Failed to serialize or deserialize state.
    Json(serde_json::Error),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "state I/O error: {e}"),
            Self::Json(e) => write!(f, "state serialization error: {e}"),
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for PersistenceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for PersistenceError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Save workflow state to a JSON file.
///
/// Uses a write-then-rename strategy so the state file is never left in a
/// partially-written state if the process crashes mid-write.
///
/// # Errors
/// Returns `PersistenceError` on I/O or serialization failure.
pub fn save_state(state: &WorkflowState, path: &Path) -> Result<(), PersistenceError> {
    let json = serde_json::to_string_pretty(state)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Load workflow state from a JSON file.
///
/// Returns `None` if the file does not exist.
///
/// # Errors
/// Returns `PersistenceError` on I/O or deserialization failure.
pub fn load_state(path: &Path) -> Result<Option<WorkflowState>, PersistenceError> {
    match std::fs::read_to_string(path) {
        Ok(json) => {
            let state: WorkflowState = serde_json::from_str(&json)?;
            Ok(Some(state))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(PersistenceError::Io(e)),
    }
}

/// Remove a state file (typically after workflow completes successfully).
///
/// # Errors
/// Returns `PersistenceError` on I/O failure. Does not error if the file
/// is already absent.
pub fn clear_state(path: &Path) -> Result<(), PersistenceError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(PersistenceError::Io(e)),
    }
}

#[cfg(test)]
mod tests;
