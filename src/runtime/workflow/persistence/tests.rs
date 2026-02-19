use super::*;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn sample_state() -> WorkflowState {
    WorkflowState {
        workflow_name: "support".to_string(),
        completed_stages: vec![CompletedStage {
            stage_name: "triage".to_string(),
            agent_name: "triage".to_string(),
            output: "Priority: high".to_string(),
            cost_cents: 5,
            tokens: 150,
        }],
        next_input: "Priority: high".to_string(),
    }
}

#[test]
fn save_and_load_roundtrip() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    let state = sample_state();
    save_state(&state, &path).unwrap();

    let loaded = load_state(&path).unwrap().expect("state should exist");
    assert_eq!(loaded, state);
}

#[test]
fn load_nonexistent_returns_none() {
    let path = PathBuf::from("/tmp/rein_test_nonexistent_state.json");
    let result = load_state(&path).unwrap();
    assert!(result.is_none());
}

#[test]
fn clear_state_removes_file() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    save_state(&sample_state(), &path).unwrap();
    assert!(path.exists());

    clear_state(&path).unwrap();
    assert!(!path.exists());
}

#[test]
fn clear_state_nonexistent_is_ok() {
    let path = PathBuf::from("/tmp/rein_test_clear_nonexistent.json");
    let result = clear_state(&path);
    assert!(result.is_ok());
}

#[test]
fn multiple_completed_stages_roundtrip() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    let state = WorkflowState {
        workflow_name: "pipeline".to_string(),
        completed_stages: vec![
            CompletedStage {
                stage_name: "a".to_string(),
                agent_name: "agent_a".to_string(),
                output: "output_a".to_string(),
                cost_cents: 3,
                tokens: 100,
            },
            CompletedStage {
                stage_name: "b".to_string(),
                agent_name: "agent_b".to_string(),
                output: "output_b".to_string(),
                cost_cents: 7,
                tokens: 200,
            },
        ],
        next_input: "output_b".to_string(),
    };

    save_state(&state, &path).unwrap();
    let loaded = load_state(&path).unwrap().unwrap();
    assert_eq!(loaded.completed_stages.len(), 2);
    assert_eq!(loaded.next_input, "output_b");
}

#[test]
fn corrupt_json_returns_error() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    std::fs::write(&path, "not valid json {{{").unwrap();
    let result = load_state(&path);
    assert!(result.is_err());
}
