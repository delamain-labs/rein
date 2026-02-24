use std::path::PathBuf;
use std::process::Command;

fn rein_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rein"))
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

// ── basic.rein ────────────────────────────────────────────────────────────────

#[test]
fn basic_exits_zero() {
    let status = Command::new(rein_bin())
        .args(["validate", example("basic.rein").to_str().unwrap()])
        .status()
        .expect("failed to spawn rein");
    assert!(status.success(), "expected exit 0 for basic.rein");
}

#[test]
fn basic_prints_valid() {
    let out = Command::new(rein_bin())
        .args(["validate", example("basic.rein").to_str().unwrap()])
        .output()
        .expect("failed to spawn rein");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Valid"),
        "expected 'Valid' in stdout, got: {stdout}"
    );
}

// ── multi_agent.rein ──────────────────────────────────────────────────────────

#[test]
fn multi_agent_exits_zero() {
    let status = Command::new(rein_bin())
        .args(["validate", example("multi_agent.rein").to_str().unwrap()])
        .status()
        .expect("failed to spawn rein");
    assert!(status.success(), "expected exit 0 for multi_agent.rein");
}

// ── invalid.rein ──────────────────────────────────────────────────────────────

#[test]
fn invalid_exits_one() {
    let out = Command::new(rein_bin())
        .args(["validate", example("invalid.rein").to_str().unwrap()])
        .output()
        .expect("failed to spawn rein");
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1 for invalid.rein"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("parse error") || stderr.contains("invalid.rein"),
        "expected 'parse error' or filename in stderr, got: {stderr}"
    );
}

// ── --ast flag ────────────────────────────────────────────────────────────────

#[test]
fn ast_flag_exits_zero_and_outputs_json() {
    let out = Command::new(rein_bin())
        .args(["validate", "--ast", example("basic.rein").to_str().unwrap()])
        .output()
        .expect("failed to spawn rein");
    assert!(out.status.success(), "expected exit 0 with --ast");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Should be a JSON object containing an "agents" array.
    assert!(
        stdout.trim_start().starts_with('{'),
        "expected JSON object in stdout, got: {stdout}"
    );
    assert!(
        stdout.contains("\"agents\""),
        "expected 'agents' key in JSON output, got: {stdout}"
    );
    assert!(
        stdout.contains("support_triage"),
        "expected agent name in JSON output, got: {stdout}"
    );
}

#[test]
fn ast_flag_multi_agent_contains_both_names() {
    let out = Command::new(rein_bin())
        .args([
            "validate",
            "--ast",
            example("multi_agent.rein").to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn rein");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("support_triage"), "missing support_triage");
    assert!(stdout.contains("billing_bot"), "missing billing_bot");
}

#[test]
fn ast_flag_invalid_file_exits_one() {
    let status = Command::new(rein_bin())
        .args([
            "validate",
            "--ast",
            example("invalid.rein").to_str().unwrap(),
        ])
        .status()
        .expect("failed to spawn rein");
    assert_eq!(
        status.code(),
        Some(1),
        "expected exit 1 for --ast on invalid.rein"
    );
}

// ── fmt command ───────────────────────────────────────────────────────────────

// #353: rein fmt must exit non-zero and report an error for syntactically
// invalid files instead of silently returning success.
#[test]
fn fmt_invalid_file_exits_nonzero() {
    let out = Command::new(rein_bin())
        .args(["fmt", example("invalid.rein").to_str().unwrap()])
        .output()
        .expect("failed to spawn rein");
    assert_ne!(
        out.status.code(),
        Some(0),
        "rein fmt must not exit 0 on an invalid file"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("syntax error") || stderr.contains("invalid.rein"),
        "rein fmt must emit a diagnostic error message to stderr, got: {stderr}"
    );
}

#[test]
fn fmt_check_invalid_file_exits_nonzero() {
    let out = Command::new(rein_bin())
        .args(["fmt", "--check", example("invalid.rein").to_str().unwrap()])
        .output()
        .expect("failed to spawn rein");
    assert_ne!(
        out.status.code(),
        Some(0),
        "rein fmt --check must not exit 0 on an invalid file"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("syntax error") || stderr.contains("invalid.rein"),
        "rein fmt --check must emit a diagnostic error message to stderr, got: {stderr}"
    );
}

// ── error paths ───────────────────────────────────────────────────────────────

#[test]
fn missing_file_exits_one() {
    let status = Command::new(rein_bin())
        .args(["validate", "no_such_file.rein"])
        .status()
        .expect("failed to spawn rein");
    assert_eq!(status.code(), Some(1), "expected exit 1 for missing file");
}

// ── eval command ──────────────────────────────────────────────────────────────

#[test]
fn eval_no_scenarios_exits_zero() {
    // basic.rein has no scenario blocks — should exit 0 cleanly
    let out = Command::new(rein_bin())
        .args(["eval", "--demo", example("basic.rein").to_str().unwrap()])
        .output()
        .expect("failed to spawn rein");
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 when no scenarios"
    );
}

#[test]
fn eval_demo_mode_all_scenarios_pass() {
    // The demo provider returns a canned response containing "customer info",
    // which satisfies the expectation in eval_scenarios.rein. Exit must be 0.
    let out = Command::new(rein_bin())
        .args([
            "eval",
            "--demo",
            example("eval_scenarios.rein").to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn rein");
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected all demo scenarios to pass (exit 0), got {} (stderr: {})",
        out.status.code().unwrap_or(101),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn eval_missing_file_exits_one() {
    let status = Command::new(rein_bin())
        .args(["eval", "--demo", "no_such_scenarios.rein"])
        .status()
        .expect("failed to spawn rein");
    assert_eq!(status.code(), Some(1), "expected exit 1 for missing file");
}

// #354: "Valid (with warnings)" must appear on stderr, not stdout.
#[test]
fn valid_with_warnings_on_stderr_not_stdout() {
    let out = Command::new(rein_bin())
        .args([
            "validate",
            "--strict",
            example("eval_scenarios.rein").to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn rein");
    assert!(out.status.success(), "expected exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stdout.contains("with warnings"),
        "'Valid (with warnings)' must not appear on stdout; stdout: {stdout}"
    );
    assert!(
        stderr.contains("with warnings"),
        "'Valid (with warnings)' must appear on stderr; stderr: {stderr}"
    );
}

#[test]
fn eval_scenario_filter_unknown_exits_zero() {
    let out = Command::new(rein_bin())
        .args([
            "eval",
            "--demo",
            "--scenario",
            "nonexistent_scenario",
            example("eval_scenarios.rein").to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn rein");
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 when named scenario not found"
    );
}

// ── #406: --stage-timeout CLI flag ────────────────────────────────────────────

#[test]
fn run_stage_timeout_flag_accepted() {
    // Verify that --stage-timeout is a recognised flag (not an "unexpected argument")
    // by running with --demo (mock provider, no API key) + --dry-run so no LLM call.
    let out = Command::new(rein_bin())
        .args([
            "run",
            "--demo",
            "--dry-run",
            "--stage-timeout",
            "30",
            example("basic.rein").to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn rein");
    assert_ne!(
        out.status.code(),
        Some(2),
        "--stage-timeout should not produce a clap parse error (exit 2); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn run_run_timeout_flag_accepted() {
    // Verify that --run-timeout is a recognised flag (not an "unexpected argument")
    // by running with --demo (mock provider, no API key) + --dry-run so no LLM call.
    let out = Command::new(rein_bin())
        .args([
            "run",
            "--demo",
            "--dry-run",
            "--run-timeout",
            "60",
            example("basic.rein").to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn rein");
    assert_ne!(
        out.status.code(),
        Some(2),
        "--run-timeout should not produce a clap parse error (exit 2); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
