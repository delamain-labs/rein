use std::sync::Arc;
use std::time::Instant;

// This function is inherently sequential setup code (parse → validate →
// build provider → attach engine extensions → dispatch). Extracting it
// further would require artificial helpers with awkward return types.
#[allow(clippy::too_many_lines)]
pub fn run_agent(
    path: &std::path::Path,
    message: Option<&str>,
    dry_run: bool,
    demo: bool,
    otel: bool,
    audit_log: Option<&std::path::Path>,
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

    let registry = rein::runtime::permissions::ToolRegistry::from_agent(agent);
    let config = rein::runtime::engine::RunConfig {
        system_prompt: None,
        max_turns: 10,
        budget_cents: agent.budget.as_ref().map_or(0, |b| b.amount),
    };

    let executor = rein::runtime::executor::NoopExecutor;
    let mut engine = rein::runtime::engine::AgentEngine::new(
        provider.as_ref(),
        &executor,
        &registry,
        Vec::new(),
        config,
    )
    .with_stream(Box::new(rein::runtime::engine::StdoutStream));

    if let Some(ref guardrails_def) = agent.guardrails {
        let guardrail_engine = rein::runtime::guardrails::GuardrailEngine::from_def(guardrails_def);
        engine = engine.with_guardrails(guardrail_engine);
    }

    if let Some(cb_def) = file.circuit_breakers.first() {
        let cb = rein::runtime::circuit_breaker::CircuitBreaker::from_def(cb_def);
        engine = engine.with_circuit_breaker(cb);
    }

    if !file.secrets.is_empty() {
        match resolve_secrets(&file.secrets) {
            Ok(map) => engine = engine.with_secrets(map),
            Err(code) => return code,
        }
    }

    // Attach policy engine if defined.
    if let Some(policy_def) = file.policies.first() {
        let policy = rein::runtime::policy::PolicyEngine::from_def(policy_def);
        eprintln!(
            "Policy: tier '{}' ({} total)",
            policy.current_tier(),
            policy.tier_count()
        );
        engine = engine.with_policy(policy);
    }

    // Prefer the observe block matching the agent name; fall back to the first.
    let obs = file
        .observes
        .iter()
        .find(|o| o.name == agent.name)
        .or_else(|| file.observes.first());
    let otel_mode = resolve_otel_mode(obs, otel);
    engine = engine
        .with_otel_mode(otel_mode)
        .with_agent_name(agent.name.clone());

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
        );
    }

    // `--audit-log` only applies to workflow runs. Single-agent runs do not
    // have a workflow context, so the audit log would record nothing useful.
    // Silently ignoring the flag would be a footgun for compliance users who
    // expect audit records but receive none — fail-hard instead.
    if audit_log.is_some() {
        eprintln!(
            "error: --audit-log requires a workflow run (use 'workflow:' in your .rein file)"
        );
        eprintln!("hint: remove --audit-log or add a workflow definition to your .rein file");
        return 1;
    }

    run_engine(&engine, user_message)
}

fn run_workflow_mode(
    workflow: &rein::ast::WorkflowDef,
    file: &rein::ast::ReinFile,
    provider: &dyn rein::runtime::provider::Provider,
    executor: &dyn rein::runtime::executor::ToolExecutor,
    budget_cents: u64,
    audit_log_path: Option<&std::path::Path>,
) -> i32 {
    // Only inject a global handler when env-var overrides are active (CI/testing).
    // In normal runs `approval_handler` is `None` so each step resolves its own
    // handler based on `ApprovalDef.channel` (CLI, Slack, webhook, …).
    let approval_handler = resolve_env_approval_handler();
    let wf_config = rein::runtime::engine::RunConfig {
        system_prompt: None,
        max_turns: 10,
        budget_cents,
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
            eprintln!();
            eprintln!(
                "--- Workflow complete ({} stages) ---",
                result.stage_results.len()
            );
            eprintln!("Final output: {}", result.final_output);
            if !result.events.is_empty() {
                eprintln!(
                    "{}",
                    rein::runtime::RunTrace::summarize_events(&result.events)
                );
            }
            eprintln!("Duration: {duration:.2?}");
            0
        }
        Err(e) => {
            eprintln!("Workflow failed: {e}");
            1
        }
    }
}

fn run_engine(engine: &rein::runtime::engine::AgentEngine<'_>, user_message: &str) -> i32 {
    let start = Instant::now();
    let result = super::provider::block_on(engine.run(user_message));

    match result {
        Ok(run_result) => {
            let duration = start.elapsed();
            eprintln!();
            eprintln!("--- Run complete ---");
            eprintln!("{}", run_result.trace.summary());
            eprintln!("Duration: {duration:.2?}");
            0
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
/// Returns a flat `HashMap<name, value>` or a CLI-friendly error code.
fn resolve_secrets(
    defs: &[rein::ast::SecretsDef],
) -> Result<std::collections::HashMap<String, String>, i32> {
    use rein::runtime::secrets::{SecretError, SecretResolver};
    let mut map = std::collections::HashMap::new();
    for def in defs {
        let secret_resolver = SecretResolver::from_def(def);
        match secret_resolver.resolve_all() {
            Ok(resolved) => {
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
                    SecretError::VaultUnavailable(path) => {
                        let env_key = format!(
                            "VAULT_{}",
                            path.chars()
                                .map(|c| if c.is_ascii_alphanumeric() {
                                    c.to_ascii_uppercase()
                                } else {
                                    '_'
                                })
                                .collect::<String>()
                        );
                        eprintln!(
                            "error: Vault path '{path}' not reachable and fallback env var '{env_key}' not set."
                        );
                        eprintln!("  → vault path: {path}");
                        eprintln!(
                            "hint: Set {env_key} as a fallback env var, or rewrite the binding to use env: source"
                        );
                    }
                }
                return Err(1);
            }
        }
    }
    Ok(map)
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
    fn audit_log_new_fails_for_unwritable_path() {
        // A path under a non-existent root cannot be created.
        let result = rein::runtime::audit::AuditLog::new(std::path::Path::new(
            "/nonexistent_root_that_cannot_exist/audit.jsonl",
        ));
        assert!(
            result.is_err(),
            "AuditLog::new should return Err for unwritable paths so CLI can fail-hard"
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
