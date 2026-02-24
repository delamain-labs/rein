use std::sync::Arc;
use std::time::Instant;

use rein::runtime::workflow::StageResult;

// This function is inherently sequential setup code (parse → validate →
// build provider → attach engine extensions → dispatch). Extracting it
// further would require artificial helpers with awkward return types.
// TODO(#460): refactor into named setup phases to remove this suppression.
// TODO(#440): extract RunOptions struct to reduce parameter count.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn run_agent(
    path: &std::path::Path,
    message: Option<&str>,
    dry_run: bool,
    demo: bool,
    otel: bool,
    audit_log: Option<&std::path::Path>,
    stage_timeout_secs: Option<u64>,
    run_timeout_secs: Option<u64>,
) -> i32 {
    let filename = path.to_string_lossy();

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{filename}': {e}");
            return 1;
        }
    };

    let file = match rein::parser::parse(&source) {
        Ok(f) => f,
        Err(e) => {
            rein::error::report_parse_error(&filename, &source, &e);
            return 1;
        }
    };

    let diags = rein::validator::validate(&file);
    let has_errors = diags.iter().any(rein::validator::Diagnostic::is_error);

    for diag in &diags {
        rein::error::report_diagnostic(&filename, &source, diag);
    }

    if has_errors {
        return 1;
    }

    if dry_run {
        return print_execution_plan(&file, message);
    }

    let Some(agent) = file.agents.first() else {
        eprintln!("error: no agents defined in '{filename}'");
        return 1;
    };

    let user_message = message.unwrap_or("Hello");

    let provider: Box<dyn rein::runtime::provider::Provider> = if demo {
        eprintln!("🎭 Demo mode: using mock provider (no API keys needed)\n");
        Box::new(rein::runtime::provider::demo::DemoProvider::new())
    } else {
        match super::provider::resolve(agent) {
            Ok(p) => p,
            Err(code) => return code,
        }
    };

    // Build permission registry from agent capabilities.
    let registry = rein::runtime::permissions::ToolRegistry::from_agent(agent);

    // Build run config. CLI flag takes precedence over DSL field.
    let resolved_stage_timeout = stage_timeout_secs.or(agent.stage_timeout_secs);
    let config = rein::runtime::engine::RunConfig {
        system_prompt: None,
        max_turns: 10,
        budget_cents: agent.budget.as_ref().map_or(0, |b| b.amount),
        stage_timeout_secs: resolved_stage_timeout,
        run_timeout_secs,
    };

    // Build engine with enforcement.
    let executor = rein::runtime::executor::NoopExecutor;
    let mut engine = rein::runtime::engine::AgentEngine::new(
        provider.as_ref(),
        &executor,
        &registry,
        Vec::new(),
        config,
    )
    .with_stream(Box::new(rein::runtime::engine::StdoutStream));

    // Attach guardrails if defined.
    if let Some(ref guardrails_def) = agent.guardrails {
        let guardrail_engine = rein::runtime::guardrails::GuardrailEngine::from_def(guardrails_def);
        engine = engine.with_guardrails(guardrail_engine);
    }

    // Attach circuit breaker if defined.
    if let Some(cb_def) = file.circuit_breakers.first() {
        let cb = rein::runtime::circuit_breaker::CircuitBreaker::from_def(cb_def);
        engine = engine.with_circuit_breaker(cb);
    }

    // Resolve and inject secrets if defined. Collect any vault-fallback events
    // so they can be prepended to the run trace for structured tracing / OTEL.
    let mut secret_fallback_events: Vec<rein::runtime::RunEvent> = Vec::new();
    if !file.secrets.is_empty() {
        match resolve_secrets(&file.secrets) {
            Ok((map, events)) => {
                engine = engine.with_secrets(map);
                secret_fallback_events = events;
            }
            Err(code) => return code,
        }
    }

    // Attach policy engine if defined.
    if let Some(policy_def) = file.policies.first() {
        let policy = rein::runtime::policy::PolicyEngine::from_def(policy_def);
        eprintln!(
            "Policy: starting at tier '{}' ({} tiers defined)",
            policy.current_tier(),
            policy.tier_count()
        );
        engine = engine.with_policy(policy);
    }

    // Resolve OTEL mode from an observe block (matched by agent name, falling back to first)
    // or the --otel flag. `observe` blocks are file-level so we prefer the one whose name
    // matches the running agent; if none match, we take the first block as a file-wide default.
    let obs = file
        .observes
        .iter()
        .find(|o| o.name == agent.name)
        .or_else(|| file.observes.first());
    let otel_mode = resolve_otel_mode(obs, otel);
    engine = engine
        .with_otel_mode(otel_mode)
        .with_agent_name(agent.name.clone());

    // `--audit-log` is only meaningful for workflow runs. Guard here — before
    // the workflow dispatch branch — so the error fires whether or not a
    // workflow is defined. Silently ignoring the flag would be a footgun for
    // compliance users who expect audit records but receive none.
    if audit_log.is_some() && file.workflows.is_empty() {
        eprintln!(
            "error: --audit-log requires a workflow run (use 'workflow:' in your .rein file)"
        );
        eprintln!("hint: remove --audit-log or add a workflow definition to your .rein file");
        return 1;
    }

    // If the file has workflows, run the first workflow instead of single-agent execution.
    if let Some(workflow) = file.workflows.first() {
        let budget_cents = agent.budget.as_ref().map_or(0, |b| b.amount);
        return run_workflow_mode(
            workflow,
            &file,
            provider.as_ref(),
            &executor,
            budget_cents,
            audit_log,
            stage_timeout_secs,
            run_timeout_secs,
            secret_fallback_events,
        );
    }

    run_engine(&engine, user_message, secret_fallback_events)
}

// TODO(#440): extract RunOptions struct to reduce parameter count.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
fn run_workflow_mode(
    workflow: &rein::ast::WorkflowDef,
    file: &rein::ast::ReinFile,
    provider: &dyn rein::runtime::provider::Provider,
    executor: &dyn rein::runtime::executor::ToolExecutor,
    budget_cents: u64,
    audit_log_path: Option<&std::path::Path>,
    stage_timeout_secs: Option<u64>,
    run_timeout_secs: Option<u64>,
    pre_events: Vec<rein::runtime::RunEvent>,
) -> i32 {
    // Only inject a global handler when env-var overrides are active (CI/testing).
    // In normal runs `approval_handler` is `None` so each step resolves its own
    // handler based on `ApprovalDef.channel` (CLI, Slack, webhook, …).
    let approval_handler = resolve_env_approval_handler();
    let wf_config = rein::runtime::engine::RunConfig {
        system_prompt: None,
        max_turns: 10,
        budget_cents,
        stage_timeout_secs,
        run_timeout_secs,
    };
    // Construct an AuditLog if the caller requested one via --audit-log.
    // Failure is fatal: an operator who explicitly passes --audit-log expects
    // full audit coverage. Silently continuing with no log would produce a
    // run with zero audit coverage while the operator believes they have it.
    let audit_log: Option<Arc<rein::runtime::audit::AuditLog>> = if let Some(p) = audit_log_path {
        match rein::runtime::audit::AuditLog::new(p) {
            Ok(log) => Some(Arc::new(log)),
            Err(e) => {
                eprintln!(
                    "error: could not initialize audit log '{}': {e}",
                    p.display()
                );
                eprintln!("hint: check that the parent directory exists and is writable");
                return 1;
            }
        }
    } else {
        None
    };

    // #411: Pre-wrap the injected approval_handler with AuditingApprovalHandler
    // before assembling WorkflowContext, so run_step receives an already-wrapped
    // handler and stays unaware of auditing logic.
    let approval_handler: Option<Arc<dyn rein::runtime::approval::ApprovalHandler>> =
        match (approval_handler, &audit_log) {
            (Some(h), Some(log)) => Some(Arc::new(
                rein::runtime::approval::AuditingApprovalHandler::with_context(
                    h,
                    Arc::clone(log),
                    Some(workflow.name.as_str()),
                    None::<&str>,
                ),
            )),
            (h, _) => h,
        };

    let ctx = rein::runtime::workflow::WorkflowContext {
        file,
        provider,
        executor,
        tool_defs: &[],
        config: &wf_config,
        approval_handler,
        audit_log,
        workflow_name: Some(workflow.name.clone()),
    };
    let start = Instant::now();
    let wf_result =
        super::provider::block_on(rein::runtime::workflow::run_workflow(workflow, &ctx));
    let duration = start.elapsed();
    match wf_result {
        Ok(result) => {
            // Count soft failures. Workflows use a partial-success model: a
            // step that fails softly (agent not found, LLM error) records a
            // StepFailed event and allows independent steps to continue rather
            // than aborting the whole run.
            //
            // Exit code convention:
            //   0 — all steps succeeded (no StepFailed or StepSkipped events)
            //   1 — partial success: at least one step failed softly or was
            //       cascade-skipped (workflow ran to completion with failures)
            //   2 — hard abort: a non-recoverable error terminated the run
            //       before all steps could execute (see Err arm below)
            let failed_count = result
                .events
                .iter()
                .filter(|e| matches!(e, rein::runtime::RunEvent::StepFailed { .. }))
                .count();
            let skipped_count = result
                .events
                .iter()
                .filter(|e| matches!(e, rein::runtime::RunEvent::StepSkipped { .. }))
                .count();

            eprintln!();
            let completed_stages = result
                .stage_results
                .iter()
                .filter(|r| r.is_real_execution())
                .count();
            eprintln!("--- Workflow complete ({completed_stages} stages) ---");
            if failed_count > 0 || skipped_count > 0 {
                // `failed_count > 0` implies `result.events` is non-empty (at least one
                // `StepFailed` event was pushed), so the trace block below always fires
                // when this warning is shown.
                eprintln!(
                    "warning: {failed_count} step(s) failed, {skipped_count} skipped (see trace below)"
                );
            }
            // Show the "all steps failed" message only when no step actually
            // ran to completion. A workflow where the terminal step produces
            // empty output (a valid LLM response) and an earlier step failed
            // must still print the real (empty) output, not the misleading
            // sentinel message.
            let has_real_execution = result
                .stage_results
                .iter()
                .any(StageResult::is_real_execution);
            if !has_real_execution && (failed_count > 0 || skipped_count > 0) {
                eprintln!("Final output: (none — all steps failed or were skipped)");
            } else {
                eprintln!("Final output: {}", result.final_output);
            }
            // Prepend secret-fallback events so they appear at the head of the
            // workflow trace, before the first step event.
            let mut display_events = pre_events;
            display_events.extend(result.events.iter().cloned());
            if !display_events.is_empty() {
                eprintln!(
                    "{}",
                    rein::runtime::RunTrace::summarize_events(&display_events)
                );
            }
            eprintln!("Duration: {duration:.2?}");
            // Exit 0: all steps succeeded.
            // Exit 1: partial success — step(s) failed, were cascade-skipped, or
            //         were conditionally skipped via when: (#455).
            // Exit 2: hard abort — see Err arm below.
            i32::from(failed_count > 0 || skipped_count > 0)
        }
        Err((e, partial_events)) => {
            eprintln!("Workflow failed: {e}");
            // Emit pre-events (e.g. SecretFallback) followed by any partial
            // events (e.g. WorkflowAborted) to stderr so the operator can see
            // the abort cause. (OTEL export is not wired to the workflow path;
            // these events do not reach an OTEL sink here.)
            let mut display_events = pre_events;
            display_events.extend(partial_events.iter().cloned());
            if !display_events.is_empty() {
                eprintln!(
                    "{}",
                    rein::runtime::RunTrace::summarize_events(&display_events)
                );
            }
            // Exit 2: hard abort (policy rejection, cyclic deps, infra failure).
            // Distinct from exit 1 (partial success) so shell consumers can
            // tell whether the workflow completed with some failures vs. was
            // terminated before all steps ran.
            2
        }
    }
}

fn run_engine(
    engine: &rein::runtime::engine::AgentEngine<'_>,
    user_message: &str,
    pre_events: Vec<rein::runtime::RunEvent>,
) -> i32 {
    let start = Instant::now();
    let result = super::provider::block_on(engine.run(user_message));

    match result {
        Ok(run_result) => {
            let duration = start.elapsed();
            eprintln!();
            eprintln!("--- Run complete ---");
            // Prepend secret-fallback events so they appear at the head of the
            // trace summary, before the first LLM call.
            let mut all_events = pre_events;
            all_events.extend(run_result.trace.events.iter().cloned());
            if !all_events.is_empty() {
                eprintln!("{}", rein::runtime::RunTrace::summarize_events(&all_events));
            }
            eprintln!("Duration: {duration:.2?}");
            0
        }
        Err(rein::runtime::RunError::Timeout { partial_trace, .. }) => {
            eprintln!();
            eprintln!(
                "Run timed out: a provider call did not respond within the configured timeout."
            );
            eprintln!("{}", partial_trace.summary());
            1
        }
        Err(e) => {
            eprintln!();
            eprintln!("Run failed: {e:?}");
            1
        }
    }
}

/// Resolve the OTEL export mode from an optional `observe` block and the `--otel` flag.
fn resolve_otel_mode(
    observe: Option<&rein::ast::ObserveDef>,
    otel_flag: bool,
) -> rein::runtime::otel_export::OtelMode {
    use rein::runtime::otel_export::OtelMode;
    if let Some(obs) = observe {
        match obs.export.as_deref() {
            Some("stdout") => {
                return OtelMode::StdoutOnComplete {
                    metrics: obs.metrics.clone(),
                };
            }
            Some("file" | "otlp") => return OtelMode::FileOnComplete,
            Some(other) => {
                eprintln!(
                    "warning: observe export target '{other}' is not supported at runtime; defaulting to file output"
                );
                return OtelMode::FileOnComplete;
            }
            None => {}
        }
    }
    if otel_flag {
        OtelMode::FileOnComplete
    } else {
        OtelMode::None
    }
}

/// Resolve all secrets from parsed `secrets { }` blocks.
///
/// Fails fast on the first unresolvable binding — subsequent bindings are not
/// checked. This is intentional to keep startup errors actionable one at a time.
///
/// Returns a flat `HashMap<name, value>` paired with any `SecretFallback` events
/// (one per vault-sourced binding that fell back to a `VAULT_*` env var).
/// Callers should prepend the returned events to the run trace for structured
/// tracing and OTEL export.
fn resolve_secrets(
    defs: &[rein::ast::SecretsDef],
) -> Result<(std::collections::HashMap<String, String>, Vec<rein::runtime::RunEvent>), i32> {
    use rein::ast::SecretSource;
    use rein::runtime::secrets::{SecretError, SecretResolver};
    let mut map = std::collections::HashMap::new();
    let mut fallback_events: Vec<rein::runtime::RunEvent> = Vec::new();
    for def in defs {
        let secret_resolver = SecretResolver::from_def(def);
        match secret_resolver.resolve_all() {
            Ok(resolved) => {
                for (name, secret) in &resolved {
                    // The warning is only set for vault→env fallbacks.
                    // Find the vault path for this binding to build a structured event.
                    if secret.warning.is_some()
                        && let Some(binding_def) =
                            def.bindings.iter().find(|b| b.name == *name)
                        && let SecretSource::Vault { path } = &binding_def.source
                    {
                        let env_key = rein::runtime::secrets::vault_env_key(path);
                        fallback_events.push(rein::runtime::RunEvent::SecretFallback {
                            binding: name.clone(),
                            vault_path: path.clone(),
                            fallback_env_var: env_key,
                        });
                    }
                }
                for (name, secret) in resolved {
                    map.insert(name, secret.value);
                }
            }
            Err(e) => {
                match &e {
                    SecretError::EnvNotFound { binding, var } => {
                        eprintln!("error: Secret binding failed.");
                        eprintln!("  → {binding} requires env var '{var}' (not set)");
                        eprintln!("hint: Add {var} to your environment or .env file");
                    }
                    SecretError::BindingNotFound(name) => {
                        eprintln!("error: Secret binding '{name}' is not configured.");
                        eprintln!(
                            "hint: Check the secrets {{ }} block for a binding named '{name}'"
                        );
                    }
                    SecretError::VaultUnavailable { path, env_key } => {
                        eprintln!(
                            "error: Vault path '{path}' not reachable and fallback env var '{env_key}' not set."
                        );
                        eprintln!("  → vault path: {path}");
                        eprintln!(
                            "hint: Set {env_key} as a fallback env var, or add real Vault integration, or rewrite the binding to use `env: {env_key}`"
                        );
                    }
                }
                return Err(1);
            }
        }
    }
    Ok((map, fallback_events))
}

/// Return a global approval handler override when an env-var is set, or `None`
/// to let each workflow step resolve its own handler from `ApprovalDef.channel`.
fn resolve_env_approval_handler() -> Option<Arc<dyn rein::runtime::approval::ApprovalHandler>> {
    if std::env::var("REIN_AUTO_APPROVE").as_deref() == Ok("1") {
        Some(Arc::new(rein::runtime::approval::AutoApproveHandler))
    } else if std::env::var("REIN_AUTO_REJECT").as_deref() == Ok("1") {
        Some(Arc::new(rein::runtime::approval::AutoRejectHandler::new(
            "auto-rejected by REIN_AUTO_REJECT",
        )))
    } else {
        None
    }
}

use super::provider::format_value_expr;

fn print_execution_plan(file: &rein::ast::ReinFile, message: Option<&str>) -> i32 {
    println!("📋 Execution Plan (dry run)\n");

    if !file.providers.is_empty() {
        println!("Providers ({}):", file.providers.len());
        for p in &file.providers {
            let model = p
                .model
                .as_ref()
                .map_or("default".to_string(), format_value_expr);
            println!("  • {} (model: {model})", p.name);
        }
        println!();
    }

    if !file.agents.is_empty() {
        println!("Agents ({}):", file.agents.len());
        for a in &file.agents {
            let can_count = a.can.len();
            let cannot_count = a.cannot.len();
            let budget_str = a.budget.as_ref().map_or_else(
                || "none".to_string(),
                |b| {
                    let sym = match b.currency.as_str() {
                        "EUR" => "€",
                        "GBP" => "£",
                        "JPY" => "¥",
                        _ => "$",
                    };
                    format!(
                        "{sym}{}.{:02} per {}",
                        b.amount / 100,
                        b.amount % 100,
                        b.unit
                    )
                },
            );
            let model = a
                .model
                .as_ref()
                .map_or("default".to_string(), format_value_expr);
            println!(
                "  • {} (model: {model}, can: {can_count}, cannot: {cannot_count}, budget: {budget_str})",
                a.name
            );
        }
        println!();
    }

    if !file.workflows.is_empty() {
        println!("Workflows ({}):", file.workflows.len());
        for w in &file.workflows {
            let trigger = &w.trigger;
            println!(
                "  • {} (trigger: {trigger}, steps: {})",
                w.name,
                w.steps.len()
            );
            for s in &w.steps {
                let guard = s
                    .when
                    .as_ref()
                    .map_or(String::new(), |_| " [guarded]".to_string());
                println!("    → {} (agent: {}){guard}", s.name, s.agent);
            }
        }
        println!();
    }

    if let Some(msg) = message {
        println!("Message: \"{msg}\"");
        println!();
    }

    println!("⚡ No API calls made. Use without --dry-run to execute.");
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rein::ast::{ObserveDef, Span};
    use rein::runtime::otel_export::OtelMode;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn obs(export: Option<&str>, metrics: &[&str]) -> ObserveDef {
        ObserveDef {
            name: "test".to_string(),
            trace: None,
            metrics: metrics.iter().map(|s| (*s).to_string()).collect(),
            alert_when: None,
            export: export.map(str::to_string),
            span: span(),
        }
    }

    #[test]
    fn resolve_otel_mode_stdout_export() {
        let o = obs(Some("stdout"), &["cost"]);
        let mode = resolve_otel_mode(Some(&o), false);
        assert!(matches!(mode, OtelMode::StdoutOnComplete { .. }));
    }

    #[test]
    fn resolve_otel_mode_file_export() {
        let o = obs(Some("file"), &[]);
        let mode = resolve_otel_mode(Some(&o), false);
        assert!(matches!(mode, OtelMode::FileOnComplete));
    }

    #[test]
    fn resolve_otel_mode_otlp_export() {
        let o = obs(Some("otlp"), &[]);
        let mode = resolve_otel_mode(Some(&o), false);
        assert!(matches!(mode, OtelMode::FileOnComplete));
    }

    #[test]
    fn resolve_otel_mode_unknown_export_falls_back_to_file() {
        let o = obs(Some("prometheus"), &[]);
        let mode = resolve_otel_mode(Some(&o), false);
        assert!(matches!(mode, OtelMode::FileOnComplete));
    }

    #[test]
    fn resolve_otel_mode_no_observe_otel_flag_true() {
        let mode = resolve_otel_mode(None, true);
        assert!(matches!(mode, OtelMode::FileOnComplete));
    }

    #[test]
    fn resolve_otel_mode_no_observe_otel_flag_false() {
        let mode = resolve_otel_mode(None, false);
        assert!(matches!(mode, OtelMode::None));
    }

    #[test]
    fn resolve_otel_mode_no_export_field_falls_through_to_flag() {
        let o = obs(None, &[]);
        let mode = resolve_otel_mode(Some(&o), true);
        assert!(matches!(mode, OtelMode::FileOnComplete));
    }

    // #358: AuditLog::new returns Err for unwritable paths. The CLI layer
    // (run_workflow_mode) is responsible for turning this Err into exit code 1;
    // this unit test verifies that AuditLog::new itself correctly fails rather
    // than silently succeeding, so the CLI logic has a reliable signal to act on.
    #[test]
    #[cfg(unix)]
    fn audit_log_new_fails_for_unwritable_path() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        // Create a tempdir, revoke all permissions, then try to create a file
        // inside it. This is hermetic and deterministic (no dependency on
        // filesystem layout or root privileges) unlike a hard-coded
        // /nonexistent-root path.
        let dir = tempfile::TempDir::new().expect("temp dir");
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o000)).expect("chmod 000");
        let result = rein::runtime::audit::AuditLog::new(dir.path().join("audit.jsonl"));
        // Restore permissions so TempDir::drop can clean up.
        let _ = fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700));
        assert!(
            result.is_err(),
            "AuditLog::new should return Err for unwritable paths so CLI can fail-hard"
        );
    }

    // #358: Passing --audit-log to a single-agent run (no workflow block) must
    // exit with code 1 and print an actionable error. Silently ignoring the flag
    // would be a footgun for compliance users who expect audit records but receive
    // none — fail-hard is the correct contract.
    #[test]
    fn run_agent_audit_log_without_workflow_exits_1() {
        use std::io::Write;
        // Minimal .rein file with an agent but no workflow block.
        let mut tmp = tempfile::NamedTempFile::new().expect("temp .rein file");
        writeln!(
            tmp,
            "agent deploy {{\n  model: \"claude-opus-4-6\"\n  goal: \"test\"\n}}"
        )
        .expect("write");
        let audit_path = tempfile::NamedTempFile::new().expect("temp audit path");
        let code = run_agent(
            tmp.path(),
            None,
            false,
            true, // demo mode — no API key needed
            false,
            Some(audit_path.path()),
            None,
            None,
        );
        assert_eq!(
            code, 1,
            "--audit-log on a single-agent run must exit 1, got {code}"
        );
    }

    // #358: The run_step production path wraps an Arc<dyn ApprovalHandler> with
    // AuditingApprovalHandler — test that the blanket impl delegates correctly.
    #[tokio::test]
    async fn auditing_handler_wraps_arc_dyn_approval_handler() {
        use rein::ast::{ApprovalDef, ApprovalKind, Span};
        use rein::runtime::approval::{ApprovalHandler, ApprovalStatus, AutoApproveHandler};
        use rein::runtime::audit::AuditLog;
        use std::sync::Arc;

        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let log = Arc::new(AuditLog::new(tmp.path()).expect("AuditLog::new"));

        // Erase to Arc<dyn ApprovalHandler> as the production path does.
        let handler: Arc<dyn ApprovalHandler> = Arc::new(AutoApproveHandler);

        let auditing =
            rein::runtime::approval::AuditingApprovalHandler::new(Arc::clone(&handler), log);

        let approval_def = ApprovalDef {
            kind: ApprovalKind::Approve,
            channel: "cli".to_string(),
            destination: String::new(),
            timeout: None,
            mode: None,
            span: Span { start: 0, end: 0 },
        };

        let status = auditing
            .request_approval("test_step", "output", &approval_def)
            .await;
        assert!(matches!(status, ApprovalStatus::Approved));
    }
}
